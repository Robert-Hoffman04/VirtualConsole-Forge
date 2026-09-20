/*
 * booter.c - freestanding second stage. Linked to run at VCF_BOOTER_ADDR
 * with its own stack (see booter.ld.in, crt0.S). No libc, no libogc.
 *
 * The loader has already staged the core's DOL at VCF_STAGE_ADDR, verified
 * with the same vcf_dol_* code that no section lands on the staged file, on
 * this region or on the launch image, and shut the system down. All that is
 * left is: copy each section to its load address, make the copy visible to
 * the instruction fetcher, and jump to the entry point.
 *
 * Option B: nothing else to do for the hand-off. The launch image already
 * sits at VCF_LAUNCH_ADDR and is not touched. (Option A would patch a
 * struct __argv into entry+8 here.)
 */
#include <stdint.h>

#include "vcf_dol.h"
#include "vcf_memmap.h"

/* dcbst each line, then sync, then icbi each line, then sync/isync: the
 * copied bytes are instructions, so data cache -> memory -> icache. */
static void sync_range(uint32_t addr, uint32_t len)
{
    uint32_t p = addr & ~31u;
    uint32_t end = (addr + len + 31u) & ~31u;

    for (; p < end; p += 32)
        __asm__ volatile("dcbst 0,%0" : : "r"(p) : "memory");
    __asm__ volatile("sync" : : : "memory");
    for (p = addr & ~31u; p < end; p += 32)
        __asm__ volatile("icbi 0,%0" : : "r"(p) : "memory");
    __asm__ volatile("sync; isync" : : : "memory");
}

/* Word copy when both sides are aligned, byte copy otherwise. Named copy_mem
 * (not memcpy) and built with -fno-tree-loop-distribute-patterns so the
 * compiler can't turn it into a call to itself. */
static void copy_mem(volatile uint8_t *dst, const volatile uint8_t *src, uint32_t n)
{
    if ((((uint32_t)dst | (uint32_t)src) & 3u) == 0) {
        volatile uint32_t *d = (volatile uint32_t *)dst;
        const volatile uint32_t *s = (const volatile uint32_t *)src;
        for (; n >= 4; n -= 4)
            *d++ = *s++;
        dst = (volatile uint8_t *)d;
        src = (const volatile uint8_t *)s;
    }
    while (n--)
        *dst++ = *src++;
}

void booter_main(uint32_t stage_len) __attribute__((noreturn));
void booter_main(uint32_t stage_len)
{
    const uint8_t *img = (const uint8_t *)VCF_STAGE_ADDR;
    VcfDol d;

    /* The loader already did this; if it somehow fails here there is nothing
     * safe to jump to and no way to report it, so park. */
    if (vcf_dol_parse(img, stage_len, &d) != VCF_DOL_OK)
        for (;;) { }

    for (int i = 0; i < VCF_DOL_SECTIONS; i++) {
        if (!vcf_dol_section_loaded(&d, i))
            continue;
        copy_mem((volatile uint8_t *)d.addr[i], img + d.off[i], d.size[i]);
        sync_range(d.addr[i], d.size[i]);
    }

    ((void (*)(void))d.entry)();
    for (;;) { }
}
