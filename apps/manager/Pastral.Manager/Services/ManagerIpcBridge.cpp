#include "ManagerIpcBridge.h"

#include "pastral_manager_ipc_bridge.h"

#include <Windows.h>

#include <array>
#include <filesystem>
#include <utility>

namespace Pastral::Manager::Services
{
    namespace
    {
        static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));

        constexpr wchar_t BridgeFileName[] = L"pastral-manager-ipc-bridge.dll";
        constexpr std::uint32_t KnownIntegrityFlags =
            PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED |
            PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK |
            PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK;

        using AbiVersionFunction = std::uint32_t (*)();
        using ResultSizeFunction = std::uint32_t (*)();
        using HealthFunction = std::int32_t (*)(
            std::uint16_t const*,
            std::size_t,
            std::uint32_t,
            PastralManagerHealthResult*);

        struct BridgeApi final
        {
            HMODULE module{};
            AbiVersionFunction abiVersion{};
            ResultSizeFunction resultSize{};
            HealthFunction health{};

            BridgeApi() = default;
            BridgeApi(BridgeApi const&) = delete;
            BridgeApi& operator=(BridgeApi const&) = delete;

            BridgeApi(BridgeApi&& other) noexcept
                : module(std::exchange(other.module, nullptr)),
                  abiVersion(std::exchange(other.abiVersion, nullptr)),
                  resultSize(std::exchange(other.resultSize, nullptr)),
                  health(std::exchange(other.health, nullptr))
            {
            }

            BridgeApi& operator=(BridgeApi&& other) noexcept
            {
                if (this != &other)
                {
                    Reset();
                    module = std::exchange(other.module, nullptr);
                    abiVersion = std::exchange(other.abiVersion, nullptr);
                    resultSize = std::exchange(other.resultSize, nullptr);
                    health = std::exchange(other.health, nullptr);
                }
                return *this;
            }

            ~BridgeApi()
            {
                Reset();
            }

            void Reset() noexcept
            {
                if (module != nullptr)
                {
                    FreeLibrary(module);
                    module = nullptr;
                }
                abiVersion = nullptr;
                resultSize = nullptr;
                health = nullptr;
            }

            [[nodiscard]] explicit operator bool() const noexcept
            {
                return module != nullptr && abiVersion != nullptr && resultSize != nullptr && health != nullptr;
            }
        };

        [[nodiscard]] std::filesystem::path ResolveBridgePath() noexcept
        {
            std::array<wchar_t, 32768> executablePath{};
            DWORD const length = GetModuleFileNameW(
                nullptr,
                executablePath.data(),
                static_cast<DWORD>(executablePath.size()));
            if (length == 0 || length >= executablePath.size())
            {
                return {};
            }

            std::filesystem::path path{ executablePath.data(), executablePath.data() + length };
            path.replace_filename(BridgeFileName);
            return path;
        }

        template <typename Function>
        [[nodiscard]] Function ResolveFunction(HMODULE module, char const* name) noexcept
        {
            return reinterpret_cast<Function>(GetProcAddress(module, name));
        }

        [[nodiscard]] BridgeApi LoadBridge() noexcept
        {
            BridgeApi api;
            auto const path = ResolveBridgePath();
            if (path.empty())
            {
                return api;
            }

            api.module = LoadLibraryExW(
                path.c_str(),
                nullptr,
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
            if (api.module == nullptr)
            {
                return api;
            }

            api.abiVersion = ResolveFunction<AbiVersionFunction>(
                api.module,
                "pastral_manager_ipc_abi_version");
            api.resultSize = ResolveFunction<ResultSizeFunction>(
                api.module,
                "pastral_manager_ipc_result_size");
            api.health = ResolveFunction<HealthFunction>(
                api.module,
                "pastral_manager_ipc_health_w");
            if (!api ||
                api.abiVersion() != PASTRAL_MANAGER_IPC_ABI_VERSION ||
                api.resultSize() != PASTRAL_MANAGER_IPC_RESULT_BYTES)
            {
                api.Reset();
            }
            return api;
        }

        [[nodiscard]] bool IsKnownStatus(std::uint32_t value) noexcept
        {
            return value <= PASTRAL_MANAGER_STATUS_ABI_MISMATCH;
        }

        [[nodiscard]] ManagerIpcBridgeHealth InvalidResult() noexcept
        {
            return {};
        }

        [[nodiscard]] ManagerIpcBridgeHealth ConvertResult(
            std::int32_t returnCode,
            PastralManagerHealthResult const& result) noexcept
        {
            if (result.abi_version != PASTRAL_MANAGER_IPC_ABI_VERSION ||
                result.struct_size != PASTRAL_MANAGER_IPC_RESULT_BYTES ||
                !IsKnownStatus(result.status) ||
                returnCode < 0 ||
                static_cast<std::uint32_t>(returnCode) != result.status ||
                result.reserved0 != 0 ||
                result.reserved1 != 0 ||
                (result.integrity_flags & ~KnownIntegrityFlags) != 0)
            {
                return InvalidResult();
            }

            auto const status = static_cast<ManagerIpcBridgeStatus>(result.status);
            if (status == ManagerIpcBridgeStatus::Connected)
            {
                auto const required =
                    PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK |
                    PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK;
                if (result.storage_schema_version == 0 ||
                    result.server_process_id == 0 ||
                    (result.integrity_flags & required) != required)
                {
                    return InvalidResult();
                }
            }
            else if (result.storage_schema_version != 0 ||
                     result.integrity_flags != 0 ||
                     result.server_process_id != 0 ||
                     result.session_id != 0 ||
                     result.connect_us != 0 ||
                     result.handshake_us != 0 ||
                     result.health_us != 0)
            {
                return InvalidResult();
            }

            ManagerIpcBridgeHealth converted;
            converted.status = status;
            converted.storageSchemaVersion = result.storage_schema_version;
            converted.captureEnabled =
                (result.integrity_flags & PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED) != 0;
            converted.privacyPolicyOk =
                (result.integrity_flags & PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK) != 0;
            converted.storageIntegrityOk =
                (result.integrity_flags & PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK) != 0;
            converted.serverProcessId = result.server_process_id;
            converted.sessionId = result.session_id;
            converted.connectMicroseconds = result.connect_us;
            converted.handshakeMicroseconds = result.handshake_us;
            converted.healthMicroseconds = result.health_us;
            return converted;
        }
    }

    bool ManagerIpcBridge::IsAvailable() noexcept
    {
        return static_cast<bool>(LoadBridge());
    }

    ManagerIpcBridgeHealth ManagerIpcBridge::QueryHealth(
        std::wstring const& dataRoot,
        std::uint32_t timeoutMilliseconds) noexcept
    {
        auto api = LoadBridge();
        if (!api)
        {
            return InvalidResult();
        }

        PastralManagerHealthResult result{};
        result.abi_version = PASTRAL_MANAGER_IPC_ABI_VERSION;
        result.struct_size = PASTRAL_MANAGER_IPC_RESULT_BYTES;
        auto const code = api.health(
            reinterpret_cast<std::uint16_t const*>(dataRoot.data()),
            dataRoot.size(),
            timeoutMilliseconds,
            &result);
        return ConvertResult(code, result);
    }
}
