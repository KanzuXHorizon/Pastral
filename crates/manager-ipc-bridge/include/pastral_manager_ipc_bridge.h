#pragma once

#include <stddef.h>
#include <stdint.h>

#define PASTRAL_MANAGER_IPC_ABI_VERSION 1u
#define PASTRAL_MANAGER_IPC_RESULT_BYTES 64u

#define PASTRAL_MANAGER_STATUS_CONNECTED 0u
#define PASTRAL_MANAGER_STATUS_DISCONNECTED 1u
#define PASTRAL_MANAGER_STATUS_TIMEOUT 2u
#define PASTRAL_MANAGER_STATUS_PROTOCOL_MISMATCH 3u
#define PASTRAL_MANAGER_STATUS_AUTHENTICATION_FAILED 4u
#define PASTRAL_MANAGER_STATUS_UNHEALTHY 5u
#define PASTRAL_MANAGER_STATUS_INVALID_ARGUMENT 6u
#define PASTRAL_MANAGER_STATUS_INTERNAL_ERROR 7u
#define PASTRAL_MANAGER_STATUS_ABI_MISMATCH 8u

#define PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED (1u << 0)
#define PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK (1u << 1)
#define PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK (1u << 2)

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

#ifdef __cplusplus
static_assert(sizeof(PastralManagerHealthResult) == 64, "Pastral manager IPC result size mismatch");
static_assert(alignof(PastralManagerHealthResult) == 8, "Pastral manager IPC result alignment mismatch");
extern "C"
{
#endif

uint32_t pastral_manager_ipc_abi_version(void);
uint32_t pastral_manager_ipc_result_size(void);
int32_t pastral_manager_ipc_health_w(
    const uint16_t* data_root,
    size_t data_root_length,
    uint32_t timeout_ms,
    PastralManagerHealthResult* result);

#ifdef __cplusplus
}
#endif
