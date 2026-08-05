#include "pch.h"
#include "ManagerDataProvider.h"
#include "ManagerIpcBridge.h"

#include <chrono>
#include <condition_variable>
#include <cwctype>
#include <filesystem>
#include <mutex>
#include <optional>
#include <thread>
#include <utility>

namespace Pastral::Manager::Presentation
{
    namespace
    {
        constexpr wchar_t DiagnosticFlagName[] = L"PASTRAL_MANAGER_DIAGNOSTIC";
        constexpr wchar_t DiagnosticRootName[] = L"PASTRAL_MANAGER_DATA_ROOT";
        constexpr wchar_t LocalAppDataName[] = L"LOCALAPPDATA";
        constexpr std::uint32_t HealthTimeoutMilliseconds = 2000;
        constexpr std::uint32_t ReadTimeoutMilliseconds = 2000;
        constexpr std::uint32_t InitialPageLimit = 50;
        constexpr DWORD MaximumEnvironmentCharacters = 32768;

        enum class RequestKind
        {
            Load,
            Refresh,
            Search,
        };

        struct PendingRequest final
        {
            std::uint64_t generation{};
            RequestKind kind{ RequestKind::Load };
            std::wstring query;
            SnapshotCompletion completion;
        };

        enum class DataRootMode
        {
            Normal,
            Diagnostic,
            Invalid,
        };

        struct DataRootResolution final
        {
            DataRootMode mode{ DataRootMode::Invalid };
            std::filesystem::path path;
        };

        [[nodiscard]] std::optional<std::wstring> ReadEnvironment(wchar_t const* name)
        {
            SetLastError(ERROR_SUCCESS);
            DWORD const required = GetEnvironmentVariableW(name, nullptr, 0);
            if (required == 0)
            {
                return GetLastError() == ERROR_ENVVAR_NOT_FOUND
                    ? std::nullopt
                    : std::optional<std::wstring>{ std::wstring{} };
            }
            if (required > MaximumEnvironmentCharacters)
            {
                return std::wstring{};
            }

            std::vector<wchar_t> buffer(required);
            DWORD const written = GetEnvironmentVariableW(name, buffer.data(), required);
            if (written == 0 || written >= required)
            {
                return std::wstring{};
            }
            return std::wstring(buffer.data(), written);
        }

        [[nodiscard]] bool IsAcceptedRoot(std::filesystem::path const& path) noexcept
        {
            try
            {
                if (path.empty() || !path.is_absolute())
                {
                    return false;
                }
                auto const native = path.native();
                return !native.empty() && native.rfind(L"\\\\", 0) != 0;
            }
            catch (...)
            {
                return false;
            }
        }

        [[nodiscard]] DataRootResolution ResolveDataRoot()
        {
            auto const diagnosticFlag = ReadEnvironment(DiagnosticFlagName);
            if (diagnosticFlag.has_value())
            {
                if (*diagnosticFlag != L"1")
                {
                    return {};
                }
                auto const diagnosticRoot = ReadEnvironment(DiagnosticRootName);
                if (!diagnosticRoot.has_value() || diagnosticRoot->empty())
                {
                    return {};
                }
                std::filesystem::path path{ *diagnosticRoot };
                if (!IsAcceptedRoot(path))
                {
                    return {};
                }
                return { DataRootMode::Diagnostic, std::move(path) };
            }

            auto const localAppData = ReadEnvironment(LocalAppDataName);
            if (!localAppData.has_value() || localAppData->empty())
            {
                return {};
            }
            std::filesystem::path path{ *localAppData };
            path /= L"Pastral";
            if (!IsAcceptedRoot(path))
            {
                return {};
            }
            return { DataRootMode::Normal, std::move(path) };
        }

        [[nodiscard]] ManagerSnapshot ErrorSnapshot(ManagerStatusCode statusCode)
        {
            ManagerSnapshot snapshot;
            snapshot.connection = ConnectionState::Error;
            snapshot.statusCode = statusCode;
            snapshot.storageSchemaVersion = 0;
            snapshot.clips.clear();
            snapshot.query.clear();
            snapshot.hasMore = false;
            snapshot.synthetic = false;
            return snapshot;
        }

        [[nodiscard]] ManagerSnapshot LiveSnapshot(
            Services::ManagerIpcBridgeHealth const& health)
        {
            ManagerSnapshot snapshot;
            snapshot.clips.clear();
            snapshot.query.clear();
            snapshot.hasMore = false;
            snapshot.synthetic = false;
            snapshot.storageSchemaVersion = health.storageSchemaVersion;

            using Services::ManagerIpcBridgeStatus;
            switch (health.status)
            {
            case ManagerIpcBridgeStatus::Connected:
                snapshot.connection = ConnectionState::Connected;
                snapshot.statusCode = ManagerStatusCode::Connected;
                break;
            case ManagerIpcBridgeStatus::Disconnected:
                snapshot.connection = ConnectionState::Disconnected;
                snapshot.statusCode = ManagerStatusCode::Disconnected;
                break;
            case ManagerIpcBridgeStatus::Timeout:
                snapshot.connection = ConnectionState::Disconnected;
                snapshot.statusCode = ManagerStatusCode::Timeout;
                break;
            case ManagerIpcBridgeStatus::ProtocolMismatch:
                snapshot.connection = ConnectionState::ProtocolMismatch;
                snapshot.statusCode = ManagerStatusCode::ProtocolMismatch;
                break;
            case ManagerIpcBridgeStatus::AuthenticationFailed:
                return ErrorSnapshot(ManagerStatusCode::AuthenticationFailed);
            case ManagerIpcBridgeStatus::Unhealthy:
                return ErrorSnapshot(ManagerStatusCode::Unhealthy);
            case ManagerIpcBridgeStatus::InvalidArgument:
                return ErrorSnapshot(ManagerStatusCode::InvalidConfiguration);
            case ManagerIpcBridgeStatus::AbiMismatch:
                return ErrorSnapshot(ManagerStatusCode::AbiMismatch);
            case ManagerIpcBridgeStatus::InsufficientBuffer:
                return ErrorSnapshot(ManagerStatusCode::HistoryChanged);
            case ManagerIpcBridgeStatus::InternalError:
            default:
                return ErrorSnapshot(ManagerStatusCode::InternalError);
            }
            return snapshot;
        }

        [[nodiscard]] std::wstring FormatUuid(
            std::array<std::uint8_t, 16> const& bytes)
        {
            constexpr wchar_t Hex[] = L"0123456789abcdef";
            std::wstring value;
            value.reserve(36);
            for (std::size_t index = 0; index < bytes.size(); ++index)
            {
                if (index == 4 || index == 6 || index == 8 || index == 10)
                {
                    value.push_back(L'-');
                }
                value.push_back(Hex[(bytes[index] >> 4) & 0x0f]);
                value.push_back(Hex[bytes[index] & 0x0f]);
            }
            return value;
        }

        [[nodiscard]] std::int64_t ObservedAtAgo(std::chrono::microseconds elapsed)
        {
            using namespace std::chrono;
            auto const now = duration_cast<microseconds>(
                system_clock::now().time_since_epoch()).count();
            return now - elapsed.count();
        }

        [[nodiscard]] ClipPreviewData MakeClip(
            std::wstring id,
            std::wstring safePreview,
            std::wstring source,
            std::int64_t observedAtUnixMicros,
            std::wstring typeLabel,
            std::wstring profile,
            std::wstring representationSummary,
            bool pinned,
            bool unavailable,
            bool previewTruncated = false)
        {
            ClipPreviewData clip;
            clip.id = std::move(id);
            clip.safePreview = std::move(safePreview);
            clip.source = std::move(source);
            clip.observedAtUnixMicros = observedAtUnixMicros;
            clip.typeLabel = std::move(typeLabel);
            clip.profile = std::move(profile);
            clip.representationSummary = std::move(representationSummary);
            clip.pinned = pinned;
            clip.unavailable = unavailable;
            clip.previewTruncated = previewTruncated;
            return clip;
        }

        [[nodiscard]] ClipPreviewData MapClip(
            Services::ManagerIpcBridgeClip const& value)
        {
            auto const unavailable = value.unavailable;
            auto const source = value.sourceLabel.has_value() && !value.sourceLabel->empty()
                ? *value.sourceLabel
                : L"Unknown source";
            std::wstring const type = unavailable ? L"Unavailable" : L"Text";
            auto const preview = unavailable
                ? L"Preview unavailable"
                : (value.preview.empty() ? L"Empty text preview" : value.preview);
            auto const representation = unavailable
                ? L"Preview metadata only · Content unavailable"
                : (value.previewTruncated ? L"Text preview · Truncated" : L"Text preview");
            return MakeClip(
                FormatUuid(value.eventId),
                preview,
                source,
                value.observedAtUnixMicros,
                type,
                L"Ordinary",
                representation,
                value.pinned,
                unavailable,
                value.previewTruncated);
        }

        [[nodiscard]] std::wstring Lowercase(std::wstring_view value)
        {
            std::wstring lowered;
            lowered.reserve(value.size());
            for (auto const character : value)
            {
                lowered.push_back(static_cast<wchar_t>(std::towlower(character)));
            }
            return lowered;
        }

        [[nodiscard]] bool SyntheticMatch(
            ClipPreviewData const& clip,
            std::wstring const& loweredQuery)
        {
            if (loweredQuery.empty())
            {
                return true;
            }
            return Lowercase(clip.safePreview).find(loweredQuery) != std::wstring::npos ||
                Lowercase(clip.source).find(loweredQuery) != std::wstring::npos ||
                Lowercase(clip.typeLabel).find(loweredQuery) != std::wstring::npos ||
                Lowercase(clip.profile).find(loweredQuery) != std::wstring::npos ||
                Lowercase(clip.representationSummary).find(loweredQuery) != std::wstring::npos;
        }

        [[nodiscard]] ManagerSnapshot SyntheticSnapshot(std::wstring const& query)
        {
            ManagerSnapshot snapshot;
            snapshot.connection = ConnectionState::Connected;
            snapshot.statusCode = ManagerStatusCode::Synthetic;
            snapshot.storageSchemaVersion = 0;
            snapshot.query = query;
            snapshot.hasMore = false;
            snapshot.synthetic = true;
            snapshot.clips = {
                MakeClip(
                    L"synthetic-clip-text",
                    L"Build verification passed for the local workspace.",
                    L"Visual Studio Code",
                    ObservedAtAgo(std::chrono::minutes(2)),
                    L"Text",
                    L"Development",
                    L"Unicode text · Plain text",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-code",
                    L"cargo test --locked --workspace --all-targets",
                    L"Windows Terminal",
                    ObservedAtAgo(std::chrono::minutes(8)),
                    L"Code",
                    L"Development",
                    L"Unicode text · Plain text",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-url",
                    L"learn.microsoft.com/windows/apps/windows-app-sdk/",
                    L"Microsoft Edge",
                    ObservedAtAgo(std::chrono::minutes(18)),
                    L"Link",
                    L"Ordinary",
                    L"Unicode text · Web link",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-image",
                    L"Screenshot · 1920 × 1080",
                    L"Snipping Tool",
                    ObservedAtAgo(std::chrono::minutes(34)),
                    L"Image",
                    L"Ordinary",
                    L"PNG · Bitmap",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-pinned",
                    L"Release checklist: verify signatures and recovery.",
                    L"Pastral Manager",
                    ObservedAtAgo(std::chrono::days(1)),
                    L"Text",
                    L"Development",
                    L"Unicode text · Plain text",
                    true,
                    false),
                MakeClip(
                    L"synthetic-clip-unavailable",
                    L"Referenced file is no longer available.",
                    L"File Explorer",
                    ObservedAtAgo(std::chrono::days(3)),
                    L"File reference",
                    L"Ordinary",
                    L"Reference only",
                    false,
                    true),
            };

            if (!query.empty())
            {
                auto const loweredQuery = Lowercase(query);
                std::vector<ClipPreviewData> filtered;
                for (auto& clip : snapshot.clips)
                {
                    if (SyntheticMatch(clip, loweredQuery))
                    {
                        filtered.push_back(std::move(clip));
                    }
                }
                snapshot.clips = std::move(filtered);
            }
            return snapshot;
        }

        [[nodiscard]] ManagerSnapshot SyntheticSnapshot()
        {
            return SyntheticSnapshot({});
        }

        [[nodiscard]] ManagerSnapshot ReadFailureSnapshot(
            Services::ManagerIpcBridgeStatus status)
        {
            Services::ManagerIpcBridgeHealth failure;
            failure.status = status;
            return LiveSnapshot(failure);
        }

        [[nodiscard]] ManagerSnapshot BuildSnapshot(
            RequestKind kind,
            std::wstring const& query)
        {
#if defined(_DEBUG)
            auto const diagnosticFlag = ReadEnvironment(DiagnosticFlagName);
            if (!diagnosticFlag.has_value())
            {
                if (kind == RequestKind::Search)
                {
                    return SyntheticSnapshot(query);
                }
                return SyntheticSnapshot();
            }
#endif
            auto const root = ResolveDataRoot();
            if (root.mode == DataRootMode::Invalid)
            {
                return ErrorSnapshot(ManagerStatusCode::InvalidConfiguration);
            }

            auto const health = Services::ManagerIpcBridge::QueryHealth(
                root.path.native(),
                HealthTimeoutMilliseconds);
            auto snapshot = LiveSnapshot(health);
            if (snapshot.connection != ConnectionState::Connected)
            {
                return snapshot;
            }
            if (!Services::ManagerIpcBridge::IsReadAvailable())
            {
                return ErrorSnapshot(ManagerStatusCode::HistoryBridgeUnavailable);
            }

            auto const page = kind == RequestKind::Search && !query.empty()
                ? Services::ManagerIpcBridge::QuerySearch(
                    root.path.native(),
                    ReadTimeoutMilliseconds,
                    query,
                    InitialPageLimit)
                : Services::ManagerIpcBridge::QueryHistory(
                    root.path.native(),
                    ReadTimeoutMilliseconds,
                    InitialPageLimit);
            if (page.status != Services::ManagerIpcBridgeStatus::Connected)
            {
                return ReadFailureSnapshot(page.status);
            }

            snapshot.query = kind == RequestKind::Search ? query : std::wstring{};
            snapshot.hasMore = page.hasMore;
            snapshot.clips.reserve(page.items.size());
            for (auto const& item : page.items)
            {
                snapshot.clips.push_back(MapClip(item));
            }
            snapshot.statusCode = page.hasMore
                ? ManagerStatusCode::ConnectedFirstPage
                : ManagerStatusCode::ConnectedCurrentPage;
            return snapshot;
        }

        class ManagerDataProvider final : public IManagerDataProvider
        {
        public:
            ManagerDataProvider()
                : m_worker([this]() noexcept { WorkerLoop(); })
            {
            }

            ManagerDataProvider(ManagerDataProvider const&) = delete;
            ManagerDataProvider& operator=(ManagerDataProvider const&) = delete;

            ~ManagerDataProvider() override
            {
                {
                    std::scoped_lock lock(m_mutex);
                    m_shutdown = true;
                    ++m_generation;
                    m_pending.reset();
                }
                m_condition.notify_one();
                if (m_worker.joinable())
                {
                    m_worker.join();
                }
            }

            void LoadSnapshotAsync(SnapshotCompletion completion) override
            {
                Queue(RequestKind::Load, {}, std::move(completion));
            }

            void RefreshAsync(SnapshotCompletion completion) override
            {
                Queue(RequestKind::Refresh, {}, std::move(completion));
            }

            void SearchAsync(std::wstring query, SnapshotCompletion completion) override
            {
                Queue(RequestKind::Search, std::move(query), std::move(completion));
            }

        private:
            void Queue(
                RequestKind kind,
                std::wstring query,
                SnapshotCompletion completion)
            {
                if (!completion)
                {
                    return;
                }
                {
                    std::scoped_lock lock(m_mutex);
                    if (m_shutdown)
                    {
                        return;
                    }
                    ++m_generation;
                    m_pending = PendingRequest{
                        m_generation,
                        kind,
                        std::move(query),
                        std::move(completion),
                    };
                }
                m_condition.notify_one();
            }

            void WorkerLoop() noexcept
            {
                for (;;)
                {
                    PendingRequest request;
                    {
                        std::unique_lock lock(m_mutex);
                        m_condition.wait(lock, [this]() noexcept {
                            return m_shutdown || m_pending.has_value();
                        });
                        if (m_shutdown)
                        {
                            return;
                        }
                        request = std::move(*m_pending);
                        m_pending.reset();
                    }

                    ManagerSnapshot snapshot;
                    try
                    {
                        snapshot = BuildSnapshot(request.kind, request.query);
                    }
                    catch (...)
                    {
                        snapshot = ErrorSnapshot(ManagerStatusCode::InternalError);
                    }

                    bool deliver = false;
                    {
                        std::scoped_lock lock(m_mutex);
                        deliver = !m_shutdown && request.generation == m_generation;
                    }
                    if (deliver)
                    {
                        try
                        {
                            request.completion(std::move(snapshot));
                        }
                        catch (...)
                        {
                        }
                    }
                }
            }

            std::mutex m_mutex;
            std::condition_variable m_condition;
            bool m_shutdown{};
            std::uint64_t m_generation{};
            std::optional<PendingRequest> m_pending;
            std::thread m_worker;
        };
    }

    ManagerSnapshot CreateLoadingSnapshot()
    {
        ManagerSnapshot snapshot;
        snapshot.connection = ConnectionState::Loading;
        snapshot.statusCode = ManagerStatusCode::Loading;
        snapshot.storageSchemaVersion = 0;
        snapshot.clips.clear();
        snapshot.query.clear();
        snapshot.hasMore = false;
        snapshot.synthetic = false;
        return snapshot;
    }

    std::shared_ptr<IManagerDataProvider> CreateManagerDataProvider()
    {
        return std::make_shared<ManagerDataProvider>();
    }
}
