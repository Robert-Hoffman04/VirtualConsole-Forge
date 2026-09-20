/*
 * vcf_dol.h - DOL header parsing and overlap checks, shared by the loader
 * (to reject a core before anything is torn down) and the booter (to copy
 * the sections). Freestanding: stdint/stddef only, no libc, no libogc.
 *
 * The header is read byte-by-byte as big-endian so the same code also runs
 * on a little-endian host in tests.
 *
 * DOL header (0x100 bytes): 7 text + 11 data sections, laid out as
 *   0x00 file offsets[18]   0x48 load addresses[18]   0x90 sizes[18]
 *   0xD8 bss address        0xDC bss size             0xE0 entry point
 */
#ifndef VCF_DOL_H
#define VCF_DOL_H

#include <stddef.h>
#include <stdint.h>

#include "vcf_memmap.h"

#define VCF_DOL_SECTIONS 18
#define VCF_DOL_HDR_SIZE 0x100

enum {
    VCF_DOL_OK        =  0,
    VCF_DOL_ERR_SHORT = -1,   /* smaller than a header */
    VCF_DOL_ERR_ELF   = -2,   /* ELF image: not supported yet */
    VCF_DOL_ERR_RANGE = -3,   /* a section's file range is outside the file */
    VCF_DOL_ERR_ENTRY = -4,   /* no entry point */
    VCF_DOL_ERR_EMPTY = -5,   /* nothing to load */
};

typedef struct {
    uint32_t off[VCF_DOL_SECTIONS];
    uint32_t addr[VCF_DOL_SECTIONS];
    uint32_t size[VCF_DOL_SECTIONS];
    uint32_t bss_addr, bss_size;
    uint32_t entry;
} VcfDol;

static inline uint32_t vcf_be32(const uint8_t *p)
{
    return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) | ((uint32_t)p[2] << 8) | p[3];
}

/* A section is loaded iff it has bytes and a sane address (the DOL spec's
 * "load address >= 0x100" rule; lower addresses are padding entries). */
static inline int vcf_dol_section_loaded(const VcfDol *d, int i)
{
    return d->size[i] != 0 && d->addr[i] >= 0x100;
}

/* Parse and bounds-check. Every loaded section's file range must lie inside
 * the `len` bytes of `img` and outside the header. */
static inline int vcf_dol_parse(const uint8_t *img, uint32_t len, VcfDol *d)
{
    if (len < VCF_DOL_HDR_SIZE)
        return VCF_DOL_ERR_SHORT;
    if (img[0] == 0x7F && img[1] == 'E' && img[2] == 'L' && img[3] == 'F')
        return VCF_DOL_ERR_ELF;

    for (int i = 0; i < VCF_DOL_SECTIONS; i++) {
        d->off[i]  = vcf_be32(img + 0x00 + 4 * i);
        d->addr[i] = vcf_be32(img + 0x48 + 4 * i);
        d->size[i] = vcf_be32(img + 0x90 + 4 * i);
    }
    d->bss_addr = vcf_be32(img + 0xD8);
    d->bss_size = vcf_be32(img + 0xDC);
    d->entry    = vcf_be32(img + 0xE0);

    int loaded = 0;
    for (int i = 0; i < VCF_DOL_SECTIONS; i++) {
        if (!vcf_dol_section_loaded(d, i))
            continue;
        loaded++;
        if (d->off[i] < VCF_DOL_HDR_SIZE || d->off[i] > len || d->size[i] > len - d->off[i])
            return VCF_DOL_ERR_RANGE;
    }
    if (loaded == 0)
        return VCF_DOL_ERR_EMPTY;
    if (d->entry == 0)
        return VCF_DOL_ERR_ENTRY;
    return VCF_DOL_OK;
}

/* Physical-address range overlap test (mask off the cached/uncached bits). */
static inline int vcf_range_overlap(uint32_t a, uint32_t alen, uint32_t b, uint32_t blen)
{
    uint64_t a0 = a & 0x3FFFFFFFu, b0 = b & 0x3FFFFFFFu;
    return alen && blen && a0 < b0 + blen && b0 < a0 + alen;
}

/* Returns the index of the first loaded section (or 18 for bss) that lands
 * on the staged file, the booter region or the launch image; -1 if none.
 * `stage_len` is the size of the staged DOL file. */
static inline int vcf_dol_first_conflict(const VcfDol *d, uint32_t stage_len)
{
    for (int i = 0; i < VCF_DOL_SECTIONS + 1; i++) {
        uint32_t a, n;
        if (i < VCF_DOL_SECTIONS) {
            if (!vcf_dol_section_loaded(d, i))
                continue;
            a = d->addr[i]; n = d->size[i];
        } else {
            a = d->bss_addr; n = d->bss_size;
        }
        if (vcf_range_overlap(a, n, VCF_STAGE_ADDR, stage_len) ||
            vcf_range_overlap(a, n, VCF_BOOTER_ADDR, VCF_BOOTER_SIZE) ||
            vcf_range_overlap(a, n, VCF_LAUNCH_ADDR, VCF_LAUNCH_MAX))
            return i;
    }
    return -1;
}

#endif /* VCF_DOL_H */
