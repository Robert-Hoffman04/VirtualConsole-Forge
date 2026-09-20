/*
 * Layout, checksum and validation tests for vcf_launch.h.
 * Built for big-endian PowerPC and run under qemu-ppc (see ../Makefile), so
 * the struct is checked in the byte order the Wii uses.
 *
 * `test_launch --dump` prints the golden image as hex; the Rust side
 * (crates/vc-core/src/launch.rs) must produce byte-identical output.
 */
#include <stdio.h>
#include <string.h>

#include "vcf_launch.h"

/* Golden inputs, mirrored in launch.rs. */
#define G_CORE    "/vcforge/cores/gb.dol"
#define G_ROM     "/vcforge/roms/test.gb"
#define G_PAYLOAD "{\"a\":1}"
#define GOLDEN_CHECKSUM 0x8858b8cdu   /* also pinned by tests/golden_launch.txt */

static uint8_t buf[VCF_LAUNCH_MAX] __attribute__((aligned(32)));
static int fails;

#define CHECK(cond) do { if (!(cond)) { printf("FAIL %s:%d  %s\n", __FILE__, __LINE__, #cond); fails++; } } while (0)

static VcfLaunchInfo *build(void)
{
    VcfLaunchInfo *li = (VcfLaunchInfo *)buf;
    uint32_t plen = (uint32_t)strlen(G_PAYLOAD);

    memset(buf, 0, sizeof buf);
    li->magic = VCF_LAUNCH_MAGIC;
    li->version = VCF_LAUNCH_VERSION;
    li->size = sizeof *li;
    li->device = VCF_DEVICE_SD;
    li->payload_size = plen;
    strcpy(li->core_path, G_CORE);
    strcpy(li->rom_path, G_ROM);
    memcpy(buf + sizeof *li, G_PAYLOAD, plen);
    li->checksum = vcf_launch_checksum(li, buf + sizeof *li, plen);
    return li;
}

int main(int argc, char **argv)
{
    VcfLaunchInfo *li = build();

    if (argc > 1 && strcmp(argv[1], "--dump") == 0) {
        uint32_t total = vcf_launch_total_size(li);
        printf("checksum=%08x\n", (unsigned)li->checksum);
        for (uint32_t i = 0; i < total; i++)
            printf("%02x", buf[i]);
        printf("\n");
        return 0;
    }

    CHECK(sizeof(VcfLaunchInfo) == 544);
    CHECK(vcf_launch_total_size(li) == 544 + 32);
    CHECK(vcf_launch_validate(li) == VCF_OK);
    CHECK(li->checksum == GOLDEN_CHECKSUM);

    /* header-only validation works before the payload has been read */
    CHECK(vcf_launch_validate_header(li) == VCF_OK);

    /* each rejection path, undone after every case */
    VcfLaunchInfo saved;
    memcpy(&saved, li, sizeof saved);
#define BREAK(stmt, want) do { stmt; CHECK(vcf_launch_validate(li) == (want)); memcpy(li, &saved, sizeof saved); } while (0)
    BREAK(li->magic = 0,                       VCF_ERR_NONE);
    BREAK(li->version = 2,                     VCF_ERR_VERSION);
    BREAK(li->flags = 1,                       VCF_ERR_VERSION);
    BREAK(li->size = 512,                      VCF_ERR_SIZE);
    BREAK(li->payload_size = VCF_LAUNCH_MAX,   VCF_ERR_SIZE);
    BREAK(li->device = 2,                      VCF_ERR_DEVICE);
    BREAK(li->core_path[0] = 'x',              VCF_ERR_PATH);   /* not absolute */
    BREAK(li->rom_path[0] = 0,                 VCF_ERR_PATH);   /* empty */
    BREAK(memset(li->rom_path, 'a', sizeof li->rom_path), VCF_ERR_PATH); /* no NUL */
    BREAK(strcpy(li->rom_path, "/"),           VCF_ERR_PATH);   /* names nothing */
    BREAK(li->reserved0 = 1,                   VCF_ERR_CHECKSUM); /* covered by checksum */
    BREAK(li->rom_path[5] ^= 1,                VCF_ERR_CHECKSUM);
    buf[sizeof *li] ^= 1;                                          /* payload byte */
    CHECK(vcf_launch_validate(li) == VCF_ERR_CHECKSUM);
    buf[sizeof *li] ^= 1;
    CHECK(vcf_launch_validate(li) == VCF_OK);

    /* stale RAM: an image whose bytes were zeroed is "none", not "corrupt" */
    memset(buf, 0, sizeof *li);
    CHECK(vcf_launch_validate(li) == VCF_ERR_NONE);

    printf(fails ? "test_launch: %d FAILED\n" : "test_launch: ok\n", fails);
    return fails != 0;
}
