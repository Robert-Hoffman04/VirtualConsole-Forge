/*
 * vcf_launch_client.h - core side of the hand-off (Option B).
 *
 * A core calls vcf_launch_take() as the FIRST thing in main(), before any
 * large allocation: libogc's malloc spills into MEM2 (MALLOC_MEM2 defaults
 * to 1), so a big heap can be handed the very memory the launch image sits
 * in. The image is copied into the caller's buffer, then consumed
 * (magic zeroed) so a later launch by another route can't replay it.
 *
 *   static uint8_t g_launch[VCF_LAUNCH_MAX] __attribute__((aligned(32)));
 *   ...
 *   const VcfLaunchInfo *li = NULL;
 *   if (vcf_launch_take(g_launch, sizeof g_launch) == VCF_OK)
 *       li = (const VcfLaunchInfo *)g_launch;   // else: defaults / menu
 *
 * Needs libogc (cache functions). The pure layout/validation lives in
 * vcf_launch.h so it stays host-testable.
 */
#ifndef VCF_LAUNCH_CLIENT_H
#define VCF_LAUNCH_CLIENT_H

#include <string.h>
#include <ogc/cache.h>

#include "vcf_launch.h"

/* dst must be 32-byte aligned and dst_cap >= the image's total size. */
static inline int vcf_launch_take(void *dst, uint32_t dst_cap)
{
    VcfLaunchInfo *src = (VcfLaunchInfo *)VCF_LAUNCH_ADDR;
    int rc;

    DCInvalidateRange(src, sizeof *src);
    rc = vcf_launch_validate_header(src);
    if (rc == VCF_OK) {
        uint32_t total = vcf_launch_total_size(src);
        if (total > dst_cap) {
            rc = VCF_ERR_SIZE;
        } else {
            DCInvalidateRange(src, total);
            rc = vcf_launch_validate(src);
            if (rc == VCF_OK)
                memcpy(dst, src, total);
        }
    }

    /* Consume once, whatever happened above. */
    src->magic = 0;
    DCFlushRange(src, sizeof *src);
    return rc;
}

#endif /* VCF_LAUNCH_CLIENT_H */
