/*
 * C interface to the Rust core (crates/ffi). Keep in sync with crates/ffi/src/lib.rs.
 */
#ifndef NEXUSFM_FFI_H
#define NEXUSFM_FFI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum NfmKind {
    NFM_KIND_FILE = 0,
    NFM_KIND_DIRECTORY = 1,
    NFM_KIND_SYMLINK = 2,
    NFM_KIND_OTHER = 3,
} NfmKind;

typedef enum NfmSortKey {
    NFM_SORT_NAME = 0,
    NFM_SORT_SIZE = 1,
    NFM_SORT_MODIFIED = 2,
} NfmSortKey;

typedef struct NfmConfig {
    bool show_hidden;
    uint8_t sort_key; /* NfmSortKey */
    bool descending;
} NfmConfig;

typedef struct NfmEntry {
    const char *name; /* valid until the listing is freed */
    const char *path; /* valid until the listing is freed */
    uint8_t kind;     /* NfmKind */
    uint64_t size;
    bool has_modified;
    int64_t modified_unix; /* seconds since the Unix epoch */
} NfmEntry;

typedef struct NfmListing NfmListing;

bool nfm_config_load(NfmConfig *out);
NfmListing *nfm_listing_open(const char *path, NfmConfig config);
const char *nfm_listing_error(const NfmListing *listing);
size_t nfm_listing_count(const NfmListing *listing);
bool nfm_listing_get(const NfmListing *listing, size_t index, NfmEntry *out);
void nfm_listing_free(NfmListing *listing);

#ifdef __cplusplus
}
#endif

#endif /* NEXUSFM_FFI_H */
