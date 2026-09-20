/*
 * main.c - the generic forwarder loader (main.dol inside every forwarder WAD).
 *
 * One precompiled copy of this DOL serves every channel. What makes a
 * channel launch a particular game is the WAD's second content, launch.cfg:
 * a launch image (see include/vcf_launch.h) naming the storage device, the
 * core DOL and the ROM, and carrying the core's config JSON as payload.
 *
 * Hand-off mechanism: OPTION B (own launch struct at a fixed address), see
 * docs/FORWARDER.md. Cores must call vcf_launch_take() first thing in main().
 *
 *   1. init video + console
 *   2. read launch.cfg from this title's own contents (ES) and validate it
 *   3. mount the device it names (SD or USB), read the core DOL into the
 *      staging region, validate the DOL and check it doesn't overlap the
 *      staging/booter/launch regions
 *   4. copy the booter and the launch image into place, flush caches
 *   5. unmount, shut the system down, jump to the booter (never returns)
 *
 * Any failure before step 5 prints a message, waits, and returns to the
 * System Menu.
 *
 * STATUS: written against the libogc/libfat headers (declarations checked),
 * NOT yet built with devkitPPC or run in Dolphin/on hardware.
 */
#include <gccore.h>
#include <fat.h>
#include <malloc.h>
#include <sdcard/wiisd_io.h>
#include <ogc/usbstorage.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "vcf_dol.h"
#include "vcf_launch.h"
#include "vcf_memmap.h"

/* libogc's malloc spills into MEM2 by default. Keep this program's heap in
 * MEM1 so it can never land on the fixed MEM2 regions in vcf_memmap.h. */
u32 MALLOC_MEM2 = 0;

/* The booter blob, embedded by `objcopy -I binary` (see Makefile). */
extern const uint8_t _binary_booter_bin_start[];
extern const uint8_t _binary_booter_bin_end[];

static void *xfb;
static GXRModeObj *rmode;

static void video_init(void)
{
    VIDEO_Init();
    rmode = VIDEO_GetPreferredMode(NULL);
    xfb = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
    console_init(xfb, 20, 20, rmode->fbWidth, rmode->xfbHeight,
                 rmode->fbWidth * VI_DISPLAY_PIX_SZ);
    VIDEO_Configure(rmode);
    VIDEO_SetNextFramebuffer(xfb);
    VIDEO_SetBlack(FALSE);
    VIDEO_Flush();
    VIDEO_WaitVSync();
    if (rmode->viTVMode & VI_NON_INTERLACE)
        VIDEO_WaitVSync();
}

/* Print, wait so the message can be read, return to the System Menu. */
static void die(const char *fmt, ...) __attribute__((noreturn, format(printf, 1, 2)));
static void die(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    printf("\n\n  ");
    vprintf(fmt, ap);
    printf("\n");
    va_end(ap);
    usleep(4 * 1000 * 1000);
    exit(1);   /* libogc's exit path returns to the System Menu */
}

/* ES_ReadContent needs a 32-byte aligned buffer, and both reads below are
 * multiples of 32 so the second buffer stays aligned. A short read is an
 * error rather than something to loop over, for the same reason. */
static void es_read_exact(s32 cfd, void *buf, uint32_t len)
{
    s32 n = ES_ReadContent(cfd, buf, len);
    if (n != (s32)len)
        die("launch.cfg: short read (%d of %u)", (int)n, (unsigned)len);
}

static VcfLaunchInfo *read_launch_cfg(void)
{
    VcfLaunchInfo *li = memalign(32, VCF_LAUNCH_MAX);
    if (!li)
        die("out of memory");

    s32 cfd = ES_OpenContent(VCF_CFG_CONTENT_INDEX);
    if (cfd < 0)
        die("cannot open launch.cfg (ES error %d)", (int)cfd);

    es_read_exact(cfd, li, sizeof *li);
    int rc = vcf_launch_validate_header(li);
    if (rc != VCF_OK)
        die("launch.cfg header invalid (%d)", rc);

    uint32_t total = vcf_launch_total_size(li);
    if (total > sizeof *li)
        es_read_exact(cfd, (uint8_t *)li + sizeof *li, total - sizeof *li);
    ES_CloseContent(cfd);

    rc = vcf_launch_validate(li);
    if (rc != VCF_OK)
        die("launch.cfg failed validation (%d)", rc);
    return li;
}

/* USB disks can take several seconds to spin up, so retry for a while. */
static const DISC_INTERFACE *mount_device(uint32_t device, const char **name_out)
{
    const DISC_INTERFACE *io = device == VCF_DEVICE_USB ? &__io_usbstorage : &__io_wiisd;
    const char *name = device == VCF_DEVICE_USB ? "usb" : "sd";
    int tries = device == VCF_DEVICE_USB ? 40 : 4;

    printf("  Mounting %s ", name);
    for (int i = 0; i < tries; i++) {
        if (fatMountSimple(name, io)) {
            printf("ok\n");
            *name_out = name;
            return io;
        }
        printf(".");
        usleep(250 * 1000);
    }
    die("cannot mount the %s device", name);
}

/* Read the core DOL into the staging region and return its length. */
static uint32_t stage_core(const char *mount, const char *path)
{
    char full[VCF_PATH_MAX + 8];
    snprintf(full, sizeof full, "%s:%s", mount, path);

    FILE *f = fopen(full, "rb");
    if (!f)
        die("cannot open core:\n  %s", full);
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    rewind(f);
    if (n < VCF_DOL_HDR_SIZE || n > VCF_STAGE_MAX) {
        fclose(f);
        die("core has a bad size (%ld bytes):\n  %s", n, full);
    }

    uint8_t *stage = (uint8_t *)VCF_STAGE_ADDR;
    size_t got = fread(stage, 1, (size_t)n, f);
    fclose(f);
    if (got != (size_t)n)
        die("short read of core:\n  %s", full);

    DCFlushRange(stage, (u32)n);
    return (uint32_t)n;
}

int main(void)
{
    video_init();
    printf("\n  VirtualConsole-Forge\n\n");

    VcfLaunchInfo *li = read_launch_cfg();
    printf("  Core: %s\n  ROM:  %s\n", li->core_path, li->rom_path);

    const char *mount = NULL;
    const DISC_INTERFACE *io = mount_device(li->device, &mount);
    uint32_t stage_len = stage_core(mount, li->core_path);

    /* Reject anything the booter couldn't safely load *before* tearing the
     * system down, while we can still tell the user why. */
    VcfDol dol;
    int rc = vcf_dol_parse((const uint8_t *)VCF_STAGE_ADDR, stage_len, &dol);
    if (rc == VCF_DOL_ERR_ELF)
        die("core is an ELF; only DOL cores are supported");
    if (rc != VCF_DOL_OK)
        die("core is not a valid DOL (%d)", rc);
    int bad = vcf_dol_first_conflict(&dol, stage_len);
    if (bad >= 0)
        die("core section %d overlaps the forwarder's reserved memory", bad);

    /* Booter into its region. */
    uint32_t booter_len = (uint32_t)(_binary_booter_bin_end - _binary_booter_bin_start);
    if (booter_len == 0 || booter_len > VCF_BOOTER_SIZE - 0x4000)
        die("embedded booter has a bad size (%u)", (unsigned)booter_len);
    memcpy((void *)VCF_BOOTER_ADDR, _binary_booter_bin_start, booter_len);
    DCFlushRange((void *)VCF_BOOTER_ADDR, booter_len);
    ICInvalidateRange((void *)VCF_BOOTER_ADDR, booter_len);

    /* Launch image, unchanged, into the hand-off region (Option B). */
    uint32_t total = vcf_launch_total_size(li);
    memcpy((void *)VCF_LAUNCH_ADDR, li, total);
    DCFlushRange((void *)VCF_LAUNCH_ADDR, total);

    /* Tear down: files, device, video. */
    fatUnmount(mount);
    io->shutdown();
    VIDEO_SetBlack(TRUE);
    VIDEO_Flush();
    VIDEO_WaitVSync();

    /* Shut libogc down (IRQs, IOS subsystems, ...), make sure interrupts are
     * off, and jump. The old __lwp_thread_stopmultitasking()/
     * __exception_closeall() pair no longer exists in current libogc; the
     * core's own startup code reinstalls exception vectors anyway. */
    SYS_ResetSystem(SYS_SHUTDOWN, 0, 0);
    IRQ_Disable();
    ((void (*)(uint32_t))VCF_BOOTER_ADDR)(stage_len);

    for (;;) { }   /* not reached */
}
