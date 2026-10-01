#ifndef QTI_RDKIT_SHIM_H
#define QTI_RDKIT_SHIM_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ABI v1: all input buffers remain owned by the caller during the call. On
// success, `png` is allocated by the shim and must be released with
// qti_rdkit_free_bytes. On failure, `error` is allocated by the shim and must
// be released with qti_rdkit_free_error.
unsigned int qti_rdkit_shim_abi_version(void);

int qti_rdkit_render_png(
    const char *smiles, size_t smiles_length, const char *legend,
    size_t legend_length, unsigned int width, unsigned int height,
    bool explicit_methyl, const int *atom_indices, size_t atom_count,
    const int *bond_indices, size_t bond_count, const double *highlight_rgb,
    bool peptide_bonds, unsigned char **png, size_t *png_length, char **error);

void qti_rdkit_free_bytes(unsigned char *value);
void qti_rdkit_free_error(char *value);

#ifdef __cplusplus
}
#endif

#endif
