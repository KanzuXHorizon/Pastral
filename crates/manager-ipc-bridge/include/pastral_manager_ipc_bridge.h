#pragma once

#include <stddef.h>
#include <stdint.h>

#define PASTRAL_MANAGER_IPC_ABI_VERSION 1u
#define PASTRAL_MANAGER_IPC_RESULT_BYTES 64u
#define PASTRAL_MANAGER_READ_ABI_VERSION 1u
#define PASTRAL_MANAGER_READ_RESULT_BYTES 64u
#define PASTRAL_MANAGER_CLIP_ITEM_BYTES 64u

#define PASTRAL_MANAGER_STATUS_CONNECTED 0u
#define PASTRAL_MANAGER_STATUS_DISCONNECTED 1u
#define PASTRAL_MANAGER_STATUS_TIMEOUT 2u
#define PASTRAL_MANAGER_STATUS_PROTOCOL_MISMATCH 3u
#define PASTRAL_MANAGER_STATUS_AUTHENTICATION_FAILED 4u
#define PASTRAL_MANAGER_STATUS_UNHEALTHY 5u
#define PASTRAL_MANAGER_STATUS_INVALID_ARGUMENT 6u
#define PASTRAL_MANAGER_STATUS_INTERNAL_ERROR 7u
#define PASTRAL_MANAGER_STATUS_ABI_MISMATCH 8u
#define PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER 9u

#define PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED (1u << 0)
#define PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK (1u << 1)
#define PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK (1u << 2)

#define PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE 0u
#define PASTRAL_MANAGER_CLIP_KIND_TEXT 1u

#define PASTRAL_MANAGER_CLIP_PINNED (1u << 0)
#define PASTRAL_MANAGER_CLIP_UNAVAILABLE (1u << 1)
#define PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED (1u << 2)

typedef struct PastralManagerHealthResult
{
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t status;
    uint32_t storage_schema_version;
    uint32_t integrity_flags;
    uint32_t server_process_id;
    uint32_t session_id;
    uint32_t reserved0;
    uint64_t connect_us;
    uint64_t handshake_us;
    uint64_t health_us;
    uint64_t reserved1;
} PastralManagerHealthResult;

typedef struct PastralManagerReadResult
{
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t status;
    uint32_t item_count;
    uint32_t has_more;
    uint32_t required_item_capacity;
    uint32_t required_text_capacity;
    uint32_t server_process_id;
    uint32_t session_id;
    uint32_t reserved0;
    uint64_t connect_us;
    uint64_t handshake_us;
    uint64_t request_us;
} PastralManagerReadResult;

typedef struct PastralManagerClipItem
{
    uint8_t event_id[16];
    uint64_t capture_order;
    int64_t observed_at_unix_micros;
    uint32_t kind;
    uint32_t flags;
    uint32_t preview_offset;
    uint32_t preview_length;
    uint32_t source_offset;
    uint32_t source_length;
    uint32_t reserved0;
    uint32_t reserved1;
} PastralManagerClipItem;

#ifdef __cplusplus
static_assert(sizeof(PastralManagerHealthResult) == 64, "Pastral manager IPC result size mismatch");
static_assert(alignof(PastralManagerHealthResult) == 8, "Pastral manager IPC result alignment mismatch");
static_assert(sizeof(PastralManagerReadResult) == 64, "Pastral manager read result size mismatch");
static_assert(alignof(PastralManagerReadResult) == 8, "Pastral manager read result alignment mismatch");
static_assert(sizeof(PastralManagerClipItem) == 64, "Pastral manager clip item size mismatch");
static_assert(alignof(PastralManagerClipItem) == 8, "Pastral manager clip item alignment mismatch");
extern "C"
{
#endif

uint32_t pastral_manager_ipc_abi_version(void);
uint32_t pastral_manager_ipc_result_size(void);
uint32_t pastral_manager_ipc_read_abi_version(void);
uint32_t pastral_manager_ipc_read_result_size(void);
uint32_t pastral_manager_ipc_clip_item_size(void);
int32_t pastral_manager_ipc_health_w(
    const uint16_t* data_root,
    size_t data_root_length,
    uint32_t timeout_ms,
    PastralManagerHealthResult* result);
int32_t pastral_manager_ipc_history_w(
    const uint16_t* data_root,
    size_t data_root_length,
    uint32_t timeout_ms,
    uint32_t limit,
    uint64_t before_capture_order,
    PastralManagerClipItem* items,
    uint32_t item_capacity,
    uint8_t* text_buffer,
    uint32_t text_capacity,
    PastralManagerReadResult* result);
int32_t pastral_manager_ipc_search_w(
    const uint16_t* data_root,
    size_t data_root_length,
    const uint16_t* query,
    size_t query_length,
    uint32_t timeout_ms,
    uint32_t limit,
    PastralManagerClipItem* items,
    uint32_t item_capacity,
    uint8_t* text_buffer,
    uint32_t text_capacity,
    PastralManagerReadResult* result);

#ifdef __cplusplus
}
#endif
