/* DOL parsing and overlap tests for vcf_dol.h (run as big-endian PPC). */
#include <stdio.h>
#include <string.h>

#include "vcf_dol.h"

static int fails;
#define CHECK(cond) do { if (!(cond)) { printf("FAIL %s:%d  %s\n", __FILE__, __LINE__, #cond); fails++; } } while (0)

static void put32(uint8_t *b, uint32_t off, uint32_t v)
{
    b[off] = v >> 24; b[off + 1] = v >> 16; b[off + 2] = v >> 8; b[off + 3] = v;
}

/* One text section at `addr`, `size` bytes, stored at file offset 0x100. */
static uint32_t make_dol(uint8_t *b, uint32_t addr, uint32_t size, uint32_t entry)
{
    memset(b, 0, 0x100 + size);
    put32(b, 0x00, 0x100);
    put32(b, 0x48, addr);
    put32(b, 0x90, size);
    put32(b, 0xE0, entry);
    return 0x100 + size;
}

static uint8_t dol[0x400];

int main(void)
{
    VcfDol d;
    uint32_t len = make_dol(dol, 0x80003100, 0x200, 0x80003100);

    CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_OK);
    CHECK(d.entry == 0x80003100 && d.addr[0] == 0x80003100 && d.size[0] == 0x200 && d.off[0] == 0x100);
    CHECK(vcf_dol_first_conflict(&d, len) == -1);

    /* structural rejections */
    CHECK(vcf_dol_parse(dol, 0xFF, &d) == VCF_DOL_ERR_SHORT);
    CHECK(vcf_dol_parse(dol, len - 1, &d) == VCF_DOL_ERR_RANGE);          /* section runs past EOF */
    uint8_t elf[0x200] = { 0x7F, 'E', 'L', 'F' };
    CHECK(vcf_dol_parse(elf, sizeof elf, &d) == VCF_DOL_ERR_ELF);
    len = make_dol(dol, 0x80003100, 0x200, 0);
    CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_ERR_ENTRY);
    len = make_dol(dol, 0x80003100, 0, 0x80003100);
    CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_ERR_EMPTY);
    len = make_dol(dol, 0x80, 0x200, 0x80003100);                         /* addr < 0x100: padding entry */
    CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_ERR_EMPTY);
    len = make_dol(dol, 0x80003100, 0x200, 0x80003100);
    put32(dol, 0x00, 0x40);                                               /* offset inside the header */
    CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_ERR_RANGE);
    put32(dol, 0x00, 0xFFFFFF00);                                         /* offset + size would wrap */
    CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_ERR_RANGE);

    /* overlap detection; stage_len = 1 MiB for these */
    const uint32_t stage_len = 0x100000;
    struct { uint32_t addr, size; int conflict; const char *what; } cases[] = {
        { 0x80003100, 0x200,      0, "MEM1, far away" },
        { 0x92000100, 0x100,      1, "inside the staged file" },
        { 0x91FFFF00, 0x200,      1, "straddles the start of the staged file" },
        { 0x91FFFF00, 0x100,      0, "ends exactly where the staged file starts" },
        { 0x92100000, 0x100,      0, "starts exactly where the staged file ends" },
        { 0x920FFFFF, 0x10,       1, "one byte into the end of the staged file" },
        { 0xD2000100, 0x100,      1, "uncached mirror of the staged file" },
        { 0x92F00000, 0x40,       1, "on the booter" },
        { 0x92F0FFF0, 0x40,       1, "on the booter stack" },
        { 0x93300800, 0x20,       1, "on the launch image" },
        { 0x933107E0, 0x20,       1, "end of the launch region" },
        { 0x93310800, 0x20,       0, "just past the launch region" },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        len = make_dol(dol, cases[i].addr, cases[i].size, 0x80003100);
        CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_OK);
        int got = vcf_dol_first_conflict(&d, stage_len) >= 0;
        if (got != cases[i].conflict)
            printf("  case: %s (got %d, want %d)\n", cases[i].what, got, cases[i].conflict);
        CHECK(got == cases[i].conflict);
    }

    /* bss is checked too */
    len = make_dol(dol, 0x80003100, 0x200, 0x80003100);
    put32(dol, 0xD8, 0x92F00000); put32(dol, 0xDC, 0x1000);
    CHECK(vcf_dol_parse(dol, len, &d) == VCF_DOL_OK);
    CHECK(vcf_dol_first_conflict(&d, stage_len) == VCF_DOL_SECTIONS);

    printf(fails ? "test_dol: %d FAILED\n" : "test_dol: ok\n", fails);
    return fails != 0;
}
