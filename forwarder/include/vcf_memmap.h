/*
 * vcf_memmap.h - fixed MEM2 regions used by the forwarder hand-off.
 *
 * Included by the loader, the booter, the linker script (via cpp) and
 * booter/crt0.S, so it must stay preprocessor-only and use plain hex
 * literals (no 'u' suffix, which the assembler rejects).
 *
 *   STAGE   raw bytes of the core's DOL file, copied here by the loader
 *   BOOTER  booter code + its stack (top of region is the initial sp)
 *   LAUNCH  the launch image (VcfLaunchInfo header + payload), Option B
 *
 * The addresses are cached-virtual (0x9xxxxxxx). Overlap checks against
 * DOL sections are done on physical addresses (mask 0x3FFFFFFF), so a DOL
 * that uses an uncached mirror of the same memory is still caught.
 */
#ifndef VCF_MEMMAP_H
#define VCF_MEMMAP_H

#define VCF_STAGE_ADDR   0x92000000
#define VCF_STAGE_MAX    0x00F00000   /* 15 MiB: stage ends where the booter begins */

#define VCF_BOOTER_ADDR  0x92F00000
#define VCF_BOOTER_SIZE  0x00010000   /* 64 KiB: code at the bottom, stack grows down from the top */

#define VCF_LAUNCH_ADDR  0x93300800
#define VCF_LAUNCH_MAX   0x00010000   /* header + payload, rounded up to 32 */

#endif /* VCF_MEMMAP_H */
