/*
 * vcf_launch.h - launch image shared by the forwarder loader and the cores.
 *
 * HAND-OFF MECHANISM: OPTION B (own launch struct at a fixed address).
 * This project does not use the argv convention. A core that may be started
 * by anything other than our loader must cope with "no launch image present"
 * and fall back to defaults.
 *
 * The same bytes travel the whole way:
 *
 *   WAD content 2 (launch.cfg)  ->  loader reads it via ES  ->  loader copies
 *   it unchanged to VCF_LAUNCH_ADDR  ->  core copies it out at the start of
 *   main() and consumes it (vcf_launch_client.h).
 *
 * Image layout (all fields big-endian PowerPC, natural alignment):
 *
 *   0x000  VcfLaunchInfo header, 544 bytes (17 * 32)
 *   0x220  payload, payload_size bytes: the core config JSON
 *          (docs/CONFIG_FORMAT.md), zero padded up to a multiple of 32
 *
 * No pointers anywhere: loader memory is gone by the time the core runs.
 * The Rust writer is crates/vc-core/src/launch.rs; keep the two in sync
 * (both sides assert the layout and share a golden checksum test).
 */
#ifndef VCF_LAUNCH_H
#define VCF_LAUNCH_H

#include <stddef.h>
#include <stdint.h>

#include "vcf_memmap.h"

#define VCF_LAUNCH_MAGIC   0x4C4E4348u   /* 'LNCH' */
#define VCF_LAUNCH_VERSION 1u
#define VCF_PATH_MAX       256           /* bytes per path field, including the NUL */

#define VCF_DEVICE_SD  0u
#define VCF_DEVICE_USB 1u

/* WAD content index of launch.cfg; content 0 is the banner, 1 is main.dol. */
#define VCF_CFG_CONTENT_INDEX 2

/* Result codes. VCF_ERR_NONE means "no launch image here" (normal for a core
 * started some other way); everything else means an image was there but bad. */
enum {
    VCF_OK           =  0,
    VCF_ERR_NONE     = -1,
    VCF_ERR_VERSION  = -2,
    VCF_ERR_SIZE     = -3,
    VCF_ERR_DEVICE   = -4,
    VCF_ERR_PATH     = -5,
    VCF_ERR_CHECKSUM = -6,
};

typedef struct __attribute__((aligned(32))) VcfLaunchInfo {
    uint32_t magic;                  /* 0x000 VCF_LAUNCH_MAGIC */
    uint32_t version;                /* 0x004 VCF_LAUNCH_VERSION */
    uint32_t size;                   /* 0x008 sizeof(VcfLaunchInfo) */
    uint32_t checksum;               /* 0x00C over header (this field skipped) + payload */
    uint32_t flags;                  /* 0x010 none defined yet, must be 0 */
    uint32_t device;                 /* 0x014 VCF_DEVICE_* holding both files below */
    uint32_t payload_size;           /* 0x018 bytes of payload after the header */
    uint32_t reserved0;              /* 0x01C */
    char     core_path[VCF_PATH_MAX];/* 0x020 core DOL, device-relative, starts with '/' */
    char     rom_path[VCF_PATH_MAX]; /* 0x120 ROM, device-relative, starts with '/' */
} VcfLaunchInfo;                     /* 0x220 = 544 */

_Static_assert(sizeof(VcfLaunchInfo) == 544, "VcfLaunchInfo size");
_Static_assert(sizeof(VcfLaunchInfo) % 32 == 0, "VcfLaunchInfo must be a multiple of 32");
_Static_assert(offsetof(VcfLaunchInfo, checksum) == 12, "checksum offset");
_Static_assert(offsetof(VcfLaunchInfo, device) == 20, "device offset");
_Static_assert(offsetof(VcfLaunchInfo, payload_size) == 24, "payload_size offset");
_Static_assert(offsetof(VcfLaunchInfo, core_path) == 32, "core_path offset");
_Static_assert(offsetof(VcfLaunchInfo, rom_path) == 288, "rom_path offset");

static inline uint32_t vcf_round32(uint32_t n) { return (n + 31u) & ~31u; }

/* Header plus payload padded to 32 bytes: what the WAD content holds and
 * what the hand-off flush/invalidate covers. Only call after the header
 * has passed vcf_launch_validate_header (payload_size is bounded there). */
static inline uint32_t vcf_launch_total_size(const VcfLaunchInfo *li)
{
    return li->size + vcf_round32(li->payload_size);
}

static inline const uint8_t *vcf_launch_payload(const VcfLaunchInfo *li)
{
    return (const uint8_t *)li + li->size;
}

/* Running checksum, sum = sum * 31 + byte. Weak on purpose: it only has to
 * catch stale or garbage RAM, not an attacker (the TMD hash already protects
 * the WAD contents). */
static inline uint32_t vcf_checksum_bytes(uint32_t sum, const uint8_t *b, uint32_t len)
{
    for (uint32_t i = 0; i < len; i++)
        sum = sum * 31u + b[i];
    return sum;
}

/* Header checksum with the checksum field itself (bytes 12..15) skipped,
 * then the payload bytes (payload_len of them, padding not included). */
static inline uint32_t vcf_launch_checksum(const VcfLaunchInfo *li,
                                           const uint8_t *payload, uint32_t payload_len)
{
    const uint8_t *b = (const uint8_t *)li;
    uint32_t sum = vcf_checksum_bytes(0, b, 12);
    sum = vcf_checksum_bytes(sum, b + 16, (uint32_t)sizeof *li - 16);
    return vcf_checksum_bytes(sum, payload, payload_len);
}

/* A path must be NUL-terminated inside its field, absolute (device-relative,
 * leading '/') and name something (not just "/"). */
static inline int vcf_path_ok(const char *p)
{
    uint32_t n = 0;
    while (n < VCF_PATH_MAX && p[n] != '\0')
        n++;
    return n >= 2 && n < VCF_PATH_MAX && p[0] == '/';
}

/* Everything that can be checked from the header alone. Safe to call before
 * the payload has been read, which is what the loader needs. */
static inline int vcf_launch_validate_header(const VcfLaunchInfo *li)
{
    if (li->magic != VCF_LAUNCH_MAGIC)          return VCF_ERR_NONE;
    if (li->version != VCF_LAUNCH_VERSION)      return VCF_ERR_VERSION;
    if (li->size != sizeof *li)                 return VCF_ERR_SIZE;
    if (li->payload_size > VCF_LAUNCH_MAX - sizeof *li) return VCF_ERR_SIZE;
    if (li->flags != 0)                         return VCF_ERR_VERSION;
    if (li->device != VCF_DEVICE_SD && li->device != VCF_DEVICE_USB) return VCF_ERR_DEVICE;
    if (!vcf_path_ok(li->core_path) || !vcf_path_ok(li->rom_path))   return VCF_ERR_PATH;
    return VCF_OK;
}

/* Full check. `li` must be followed in memory by its payload. */
static inline int vcf_launch_validate(const VcfLaunchInfo *li)
{
    int rc = vcf_launch_validate_header(li);
    if (rc != VCF_OK)
        return rc;
    if (li->checksum != vcf_launch_checksum(li, vcf_launch_payload(li), li->payload_size))
        return VCF_ERR_CHECKSUM;
    return VCF_OK;
}

#endif /* VCF_LAUNCH_H */
