/*
 * check_launch_cfg <file>: validate a launch.cfg produced by the Rust tool
 * with the same C code the loader runs on the console, and print what the
 * loader/core would see. Exit status 0 = the loader would accept it.
 * (`make check-cfg CFG=path/to/launch.cfg`)
 */
#include <stdio.h>
#include <string.h>

#include "vcf_launch.h"

static uint8_t buf[VCF_LAUNCH_MAX] __attribute__((aligned(32)));

int main(int argc, char **argv)
{
    if (argc != 2) { fprintf(stderr, "usage: %s launch.cfg\n", argv[0]); return 2; }
    FILE *f = fopen(argv[1], "rb");
    if (!f) { perror(argv[1]); return 2; }
    size_t n = fread(buf, 1, sizeof buf, f);
    fclose(f);

    const VcfLaunchInfo *li = (const VcfLaunchInfo *)buf;
    if (n < sizeof *li) { printf("too short: %zu bytes\n", n); return 1; }

    int rc = vcf_launch_validate_header(li);
    if (rc == VCF_OK && n < vcf_launch_total_size(li)) rc = VCF_ERR_SIZE;
    if (rc == VCF_OK) rc = vcf_launch_validate(li);
    if (rc != VCF_OK) { printf("REJECTED by the loader's checks (%d)\n", rc); return 1; }

    printf("accepted: %zu bytes, total %u\n", n, (unsigned)vcf_launch_total_size(li));
    printf("  device:  %s\n", li->device == VCF_DEVICE_USB ? "usb" : "sd");
    printf("  core:    %s\n  rom:     %s\n", li->core_path, li->rom_path);
    printf("  payload: %u bytes: %.60s%s\n", (unsigned)li->payload_size,
           (const char *)vcf_launch_payload(li), li->payload_size > 60 ? "..." : "");
    return 0;
}
