#include "ManagerIpcBridge.h"

#include "pastral_manager_ipc_bridge.h"

#include <Windows.h>

#include <algorithm>
#include <array>
#include <filesystem>
#include <limits>
#include <utility>
#include <vector>

namespace Pastral::Manager::Services
{
    namespace
    {
        static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));

        constexpr wchar_t BridgeFileName[] = L"pastral-manager-ipc-bridge.dll";
        constexpr std::uint32_t MaxReadItems = 100;
        constexpr std::uint32_t MaxTextBytes = 256 * 1024;
        constexpr std::int64_t MinUtcUnixMicros = -62'135'596'800'000'000;
        constexpr std::int64_t MaxUtcUnixMicros = 253'402'300'799'999'999;
        constexpr std::uint32_t KnownIntegrityFlags =
            PASTRAL_MANAGER_HEALTH_CAPTURE_ENABLED |
            PASTRAL_MANAGER_HEALTH_PRIVACY_POLICY_OK |
            PASTRAL_MANAGER_HEALTH_STORAGE_INTEGRITY_OK;
        constexpr std::uint32_t KnownClipFlags =
            PASTRAL_MANAGER_CLIP_PINNED |
            PASTRAL_MANAGER_CLIP_UNAVAILABLE |
            PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED;

        using AbiVersionFunction = std::uint32_t (*)();
        using ResultSizeFunction = std::uint32_t (*)();
        using HealthFunction = std::int32_t (*)(
            std::uint16_t const*,
            std::size_t,
            std::uint32_t,
            PastralManagerHealthResult*);
        using HistoryFunction = std::int32_t (*)(
            std::uint16_t const*,
            std::size_t,
            std::uint32_t,
            std::uint32_t,
            std::uint64_t,
            PastralManagerClipItem*,
            std::uint32_t,
            std::uint8_t*,
            std::uint32_t,
            PastralManagerReadResult*);
        using SearchFunction = std::int32_t (*)(
            std::uint16_t const*,
            std::size_t,
            std::uint16_t const*,
            std::size_t,
            std::uint32_t,
            std::uint32_t,
            PastralManagerClipItem*,
            std::uint32_t,
            std::uint8_t*,
            std::uint32_t,
            PastralManagerReadResult*);

        struct BridgeApi final
        {
            HMODULE module{};
            AbiVersionFunction abiVersion{};
            ResultSizeFunction resultSize{};
            HealthFunction health{};
            AbiVersionFunction readAbiVersion{};
            ResultSizeFunction readResultSize{};
            ResultSizeFunction clipItemSize{};
            HistoryFunction history{};
            SearchFunction search{};

            BridgeApi() = default;
            BridgeApi(BridgeApi const&) = delete;
            BridgeApi& operator=(BridgeApi const&) = delete;

            BridgeApi(BridgeApi&& other) noexcept
                : module(std::exchange(other.module, nullptr)),
                  abiVersion(std::exchange(other.abiVersion, nullptr)),
                  resultSize(std::exchange(other.resultSize, nullptr)),
                  health(std::exchange(other.health, nullptr)),
                  readAbiVersion(std::exchange(other.readAbiVersion, nullptr)),
                  readResultSize(std::exchange(other.readResultSize, nullptr)),
                  clipItemSize(std::exchange(other.clipItemSize, nullptr)),
                  history(std::exchange(other.history, nullptr)),
                  search(std::exchange(other.search, nullptr))
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
                    readAbiVersion = std::exchange(other.readAbiVersion, nullptr);
                    readResultSize = std::exchange(other.readResultSize, nullptr);
                    clipItemSize = std::exchange(other.clipItemSize, nullptr);
                    history = std::exchange(other.history, nullptr);
                    search = std::exchange(other.search, nullptr);
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
                ClearRead();
            }

            void ClearRead() noexcept
            {
                readAbiVersion = nullptr;
                readResultSize = nullptr;
                clipItemSize = nullptr;
                history = nullptr;
                search = nullptr;
            }

            [[nodiscard]] bool HealthAvailable() const noexcept
            {
                return module != nullptr && abiVersion != nullptr && resultSize != nullptr && health != nullptr;
            }

            [[nodiscard]] bool ReadAvailable() const noexcept
            {
                return HealthAvailable() &&
                    readAbiVersion != nullptr &&
                    readResultSize != nullptr &&
                    clipItemSize != nullptr &&
                    history != nullptr &&
                    search != nullptr;
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
            try
            {
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
                if (!api.HealthAvailable() ||
                    api.abiVersion() != PASTRAL_MANAGER_IPC_ABI_VERSION ||
                    api.resultSize() != PASTRAL_MANAGER_IPC_RESULT_BYTES)
                {
                    api.Reset();
                    return api;
                }

                api.readAbiVersion = ResolveFunction<AbiVersionFunction>(
                    api.module,
                    "pastral_manager_ipc_read_abi_version");
                api.readResultSize = ResolveFunction<ResultSizeFunction>(
                    api.module,
                    "pastral_manager_ipc_read_result_size");
                api.clipItemSize = ResolveFunction<ResultSizeFunction>(
                    api.module,
                    "pastral_manager_ipc_clip_item_size");
                api.history = ResolveFunction<HistoryFunction>(
                    api.module,
                    "pastral_manager_ipc_history_w");
                api.search = ResolveFunction<SearchFunction>(
                    api.module,
                    "pastral_manager_ipc_search_w");
                if (!api.ReadAvailable() ||
                    api.readAbiVersion() != PASTRAL_MANAGER_READ_ABI_VERSION ||
                    api.readResultSize() != PASTRAL_MANAGER_READ_RESULT_BYTES ||
                    api.clipItemSize() != PASTRAL_MANAGER_CLIP_ITEM_BYTES)
                {
                    api.ClearRead();
                }
                return api;
            }
            catch (...)
            {
                api.Reset();
                return api;
            }
        }

        [[nodiscard]] bool IsKnownHealthStatus(std::uint32_t value) noexcept
        {
            return value <= PASTRAL_MANAGER_STATUS_ABI_MISMATCH;
        }

        [[nodiscard]] bool IsKnownReadStatus(std::uint32_t value) noexcept
        {
            return value <= PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER;
        }

        [[nodiscard]] ManagerIpcBridgeHealth InvalidHealth() noexcept
        {
            return {};
        }

        [[nodiscard]] ManagerIpcBridgePage InvalidPage() noexcept
        {
            return {};
        }

        [[nodiscard]] ManagerIpcBridgeHealth ConvertHealthResult(
            std::int32_t returnCode,
            PastralManagerHealthResult const& result) noexcept
        {
            if (result.abi_version != PASTRAL_MANAGER_IPC_ABI_VERSION ||
                result.struct_size != PASTRAL_MANAGER_IPC_RESULT_BYTES ||
                !IsKnownHealthStatus(result.status) ||
                returnCode < 0 ||
                static_cast<std::uint32_t>(returnCode) != result.status ||
                result.reserved0 != 0 ||
                result.reserved1 != 0 ||
                (result.integrity_flags & ~KnownIntegrityFlags) != 0)
            {
                return InvalidHealth();
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
                    return InvalidHealth();
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
                return InvalidHealth();
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

        [[nodiscard]] bool ValidateReadEnvelope(
            std::int32_t returnCode,
            PastralManagerReadResult const& result) noexcept
        {
            return result.abi_version == PASTRAL_MANAGER_READ_ABI_VERSION &&
                result.struct_size == PASTRAL_MANAGER_READ_RESULT_BYTES &&
                IsKnownReadStatus(result.status) &&
                returnCode >= 0 &&
                static_cast<std::uint32_t>(returnCode) == result.status &&
                result.reserved0 == 0;
        }

        [[nodiscard]] bool ValidateFailedReadResult(
            PastralManagerReadResult const& result) noexcept
        {
            if (result.status == PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER)
            {
                return result.item_count == 0 &&
                    result.has_more == 0 &&
                    result.required_item_capacity <= MaxReadItems &&
                    result.required_text_capacity <= MaxTextBytes &&
                    result.server_process_id == 0 &&
                    result.session_id == 0 &&
                    result.connect_us == 0 &&
                    result.handshake_us == 0 &&
                    result.request_us == 0;
            }
            return result.status != PASTRAL_MANAGER_STATUS_CONNECTED &&
                result.item_count == 0 &&
                result.has_more == 0 &&
                result.required_item_capacity == 0 &&
                result.required_text_capacity == 0 &&
                result.server_process_id == 0 &&
                result.session_id == 0 &&
                result.connect_us == 0 &&
                result.handshake_us == 0 &&
                result.request_us == 0;
        }

        struct TextRange final
        {
            std::size_t begin{};
            std::size_t end{};
        };

        [[nodiscard]] bool AddTextRange(
            std::uint32_t offset,
            std::uint32_t length,
            std::size_t textSize,
            std::vector<TextRange>& ranges) noexcept
        {
            if (length == 0)
            {
                return true;
            }
            auto const begin = static_cast<std::size_t>(offset);
            auto const count = static_cast<std::size_t>(length);
            if (begin > textSize || count > textSize - begin)
            {
                return false;
            }
            TextRange const candidate{ begin, begin + count };
            for (auto const& range : ranges)
            {
                if (candidate.begin < range.end && range.begin < candidate.end)
                {
                    return false;
                }
            }
            ranges.push_back(candidate);
            return true;
        }

        [[nodiscard]] bool ConvertUtf8(
            std::vector<std::uint8_t> const& text,
            std::uint32_t offset,
            std::uint32_t length,
            std::wstring& converted) noexcept
        {
            if (length == 0)
            {
                converted.clear();
                return true;
            }
            if (length > static_cast<std::uint32_t>(std::numeric_limits<int>::max()))
            {
                return false;
            }
            auto const* bytes = reinterpret_cast<char const*>(text.data() + offset);
            int const inputLength = static_cast<int>(length);
            int const required = MultiByteToWideChar(
                CP_UTF8,
                MB_ERR_INVALID_CHARS,
                bytes,
                inputLength,
                nullptr,
                0);
            if (required <= 0)
            {
                return false;
            }
            converted.resize(static_cast<std::size_t>(required));
            int const written = MultiByteToWideChar(
                CP_UTF8,
                MB_ERR_INVALID_CHARS,
                bytes,
                inputLength,
                converted.data(),
                required);
            return written == required;
        }

        [[nodiscard]] ManagerIpcBridgePage ConvertConnectedPage(
            PastralManagerReadResult const& result,
            std::vector<PastralManagerClipItem> const& rawItems,
            std::vector<std::uint8_t> const& text) noexcept
        {
            try
            {
                if (result.status != PASTRAL_MANAGER_STATUS_CONNECTED ||
                    result.item_count > rawItems.size() ||
                    result.item_count > MaxReadItems ||
                    result.has_more > 1 ||
                    result.required_item_capacity != result.item_count ||
                    result.required_text_capacity > text.size() ||
                    result.required_text_capacity > MaxTextBytes ||
                    result.server_process_id == 0)
                {
                    return InvalidPage();
                }

                ManagerIpcBridgePage page;
                page.status = ManagerIpcBridgeStatus::Connected;
                page.hasMore = result.has_more != 0;
                page.serverProcessId = result.server_process_id;
                page.sessionId = result.session_id;
                page.connectMicroseconds = result.connect_us;
                page.handshakeMicroseconds = result.handshake_us;
                page.requestMicroseconds = result.request_us;
                page.items.reserve(result.item_count);
                std::vector<TextRange> ranges;
                ranges.reserve(static_cast<std::size_t>(result.item_count) * 2);

                for (std::uint32_t index = 0; index < result.item_count; ++index)
                {
                    auto const& raw = rawItems[index];
                    if (raw.reserved0 != 0 ||
                        raw.reserved1 != 0 ||
                        raw.capture_order == 0 ||
                        raw.observed_at_unix_micros < MinUtcUnixMicros ||
                        raw.observed_at_unix_micros > MaxUtcUnixMicros ||
                        (raw.flags & ~KnownClipFlags) != 0 ||
                        (raw.kind != PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE &&
                         raw.kind != PASTRAL_MANAGER_CLIP_KIND_TEXT) ||
                        std::all_of(raw.event_id, raw.event_id + 16, [](std::uint8_t value) { return value == 0; }) ||
                        !AddTextRange(raw.preview_offset, raw.preview_length, text.size(), ranges) ||
                        !AddTextRange(raw.source_offset, raw.source_length, text.size(), ranges) ||
                        (raw.source_length == 0 && raw.source_offset != 0))
                    {
                        return InvalidPage();
                    }

                    bool const unavailable = (raw.flags & PASTRAL_MANAGER_CLIP_UNAVAILABLE) != 0;
                    if ((raw.kind == PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE) != unavailable ||
                        (unavailable && raw.preview_length != 0))
                    {
                        return InvalidPage();
                    }

                    ManagerIpcBridgeClip clip;
                    std::copy_n(raw.event_id, clip.eventId.size(), clip.eventId.begin());
                    clip.captureOrder = raw.capture_order;
                    clip.observedAtUnixMicros = raw.observed_at_unix_micros;
                    clip.kind = static_cast<ManagerIpcBridgeClipKind>(raw.kind);
                    if (!ConvertUtf8(text, raw.preview_offset, raw.preview_length, clip.preview))
                    {
                        return InvalidPage();
                    }
                    if (raw.source_length != 0)
                    {
                        std::wstring source;
                        if (!ConvertUtf8(text, raw.source_offset, raw.source_length, source))
                        {
                            return InvalidPage();
                        }
                        clip.sourceLabel = std::move(source);
                    }
                    clip.pinned = (raw.flags & PASTRAL_MANAGER_CLIP_PINNED) != 0;
                    clip.unavailable = unavailable;
                    clip.previewTruncated =
                        (raw.flags & PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED) != 0;
                    page.items.push_back(std::move(clip));
                }
                return page;
            }
            catch (...)
            {
                return InvalidPage();
            }
        }

        [[nodiscard]] ManagerIpcBridgePage FailedPage(
            PastralManagerReadResult const& result) noexcept
        {
            if (!ValidateFailedReadResult(result) ||
                result.status == PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER)
            {
                return InvalidPage();
            }
            ManagerIpcBridgePage page;
            page.status = static_cast<ManagerIpcBridgeStatus>(result.status);
            return page;
        }

        template <typename Operation>
        [[nodiscard]] ManagerIpcBridgePage QueryPage(Operation&& operation) noexcept
        {
            try
            {
                PastralManagerReadResult sizing{};
                sizing.abi_version = PASTRAL_MANAGER_READ_ABI_VERSION;
                sizing.struct_size = PASTRAL_MANAGER_READ_RESULT_BYTES;
                auto sizingCode = operation(nullptr, 0, nullptr, 0, &sizing);
                if (!ValidateReadEnvelope(sizingCode, sizing))
                {
                    return InvalidPage();
                }
                if (sizing.status == PASTRAL_MANAGER_STATUS_CONNECTED)
                {
                    std::vector<PastralManagerClipItem> emptyItems;
                    std::vector<std::uint8_t> emptyText;
                    return ConvertConnectedPage(sizing, emptyItems, emptyText);
                }
                if (sizing.status != PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER ||
                    !ValidateFailedReadResult(sizing) ||
                    sizing.required_item_capacity == 0 ||
                    sizing.required_item_capacity > MaxReadItems ||
                    sizing.required_text_capacity > MaxTextBytes)
                {
                    return FailedPage(sizing);
                }

                auto itemCapacity = sizing.required_item_capacity;
                auto textCapacity = sizing.required_text_capacity;
                for (int attempt = 0; attempt < 2; ++attempt)
                {
                    std::vector<PastralManagerClipItem> items(itemCapacity);
                    std::vector<std::uint8_t> text(textCapacity);
                    PastralManagerReadResult result{};
                    result.abi_version = PASTRAL_MANAGER_READ_ABI_VERSION;
                    result.struct_size = PASTRAL_MANAGER_READ_RESULT_BYTES;
                    auto const code = operation(
                        items.data(),
                        itemCapacity,
                        text.data(),
                        textCapacity,
                        &result);
                    if (!ValidateReadEnvelope(code, result))
                    {
                        return InvalidPage();
                    }
                    if (result.status == PASTRAL_MANAGER_STATUS_CONNECTED)
                    {
                        return ConvertConnectedPage(result, items, text);
                    }
                    if (result.status != PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER ||
                        attempt != 0 ||
                        !ValidateFailedReadResult(result) ||
                        result.required_item_capacity == 0 ||
                        result.required_item_capacity > MaxReadItems ||
                        result.required_text_capacity > MaxTextBytes ||
                        (result.required_item_capacity == itemCapacity &&
                         result.required_text_capacity == textCapacity))
                    {
                        return FailedPage(result);
                    }
                    itemCapacity = result.required_item_capacity;
                    textCapacity = result.required_text_capacity;
                }
                return InvalidPage();
            }
            catch (...)
            {
                return InvalidPage();
            }
        }
    }

    bool ManagerIpcBridge::IsAvailable() noexcept
    {
        return LoadBridge().HealthAvailable();
    }

    bool ManagerIpcBridge::IsReadAvailable() noexcept
    {
        return LoadBridge().ReadAvailable();
    }

    ManagerIpcBridgeHealth ManagerIpcBridge::QueryHealth(
        std::wstring const& dataRoot,
        std::uint32_t timeoutMilliseconds) noexcept
    {
        auto api = LoadBridge();
        if (!api.HealthAvailable())
        {
            return InvalidHealth();
        }

        PastralManagerHealthResult result{};
        result.abi_version = PASTRAL_MANAGER_IPC_ABI_VERSION;
        result.struct_size = PASTRAL_MANAGER_IPC_RESULT_BYTES;
        auto const code = api.health(
            reinterpret_cast<std::uint16_t const*>(dataRoot.data()),
            dataRoot.size(),
            timeoutMilliseconds,
            &result);
        return ConvertHealthResult(code, result);
    }

    ManagerIpcBridgePage ManagerIpcBridge::QueryHistory(
        std::wstring const& dataRoot,
        std::uint32_t timeoutMilliseconds,
        std::uint32_t limit,
        std::optional<std::uint64_t> beforeCaptureOrder) noexcept
    {
        auto api = LoadBridge();
        if (!api.ReadAvailable())
        {
            return InvalidPage();
        }
        return QueryPage([&](
            PastralManagerClipItem* items,
            std::uint32_t itemCapacity,
            std::uint8_t* text,
            std::uint32_t textCapacity,
            PastralManagerReadResult* result) {
            return api.history(
                reinterpret_cast<std::uint16_t const*>(dataRoot.data()),
                dataRoot.size(),
                timeoutMilliseconds,
                limit,
                beforeCaptureOrder.value_or(0),
                items,
                itemCapacity,
                text,
                textCapacity,
                result);
        });
    }

    ManagerIpcBridgePage ManagerIpcBridge::QuerySearch(
        std::wstring const& dataRoot,
        std::uint32_t timeoutMilliseconds,
        std::wstring const& query,
        std::uint32_t limit) noexcept
    {
        auto api = LoadBridge();
        if (!api.ReadAvailable())
        {
            return InvalidPage();
        }
        return QueryPage([&](
            PastralManagerClipItem* items,
            std::uint32_t itemCapacity,
            std::uint8_t* text,
            std::uint32_t textCapacity,
            PastralManagerReadResult* result) {
            return api.search(
                reinterpret_cast<std::uint16_t const*>(dataRoot.data()),
                dataRoot.size(),
                reinterpret_cast<std::uint16_t const*>(query.data()),
                query.size(),
                timeoutMilliseconds,
                limit,
                items,
                itemCapacity,
                text,
                textCapacity,
                result);
        });
    }
}
