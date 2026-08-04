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

        [[nodiscard]] ManagerSnapshot ErrorSnapshot(
            std::wstring title,
            std::wstring detail)
        {
            ManagerSnapshot snapshot;
            snapshot.connection = ConnectionState::Error;
            snapshot.statusTitle = std::move(title);
            snapshot.statusDetail = std::move(detail);
            snapshot.activeProfile = L"Ordinary";
            snapshot.storageSummary = L"Unavailable until the local agent is healthy";
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
            snapshot.activeProfile = L"Ordinary";
            snapshot.clips.clear();
            snapshot.query.clear();
            snapshot.hasMore = false;
            snapshot.synthetic = false;

            using Services::ManagerIpcBridgeStatus;
            switch (health.status)
            {
            case ManagerIpcBridgeStatus::Connected:
                snapshot.connection = ConnectionState::Connected;
                snapshot.statusTitle = L"Pastral agent is connected";
                snapshot.statusDetail =
                    L"The secure local connection is ready and storage checks passed.";
                snapshot.storageSummary =
                    L"Schema " + std::to_wstring(health.storageSchemaVersion) + L" · Integrity verified";
                break;
            case ManagerIpcBridgeStatus::Disconnected:
                snapshot.connection = ConnectionState::Disconnected;
                snapshot.statusTitle = L"Pastral agent is not connected";
                snapshot.statusDetail =
                    L"Start the local agent, then retry the authenticated connection.";
                snapshot.storageSummary = L"Unavailable until the local agent is connected";
                break;
            case ManagerIpcBridgeStatus::Timeout:
                snapshot.connection = ConnectionState::Disconnected;
                snapshot.statusTitle = L"Pastral agent did not respond";
                snapshot.statusDetail =
                    L"The local agent took too long to respond. Check it, then retry.";
                snapshot.storageSummary = L"Unavailable because the local agent timed out";
                break;
            case ManagerIpcBridgeStatus::ProtocolMismatch:
                snapshot.connection = ConnectionState::ProtocolMismatch;
                snapshot.statusTitle = L"Pastral versions are incompatible";
                snapshot.statusDetail =
                    L"The manager and local agent use incompatible connection versions.";
                snapshot.storageSummary = L"Unavailable until manager and agent versions match";
                break;
            case ManagerIpcBridgeStatus::AuthenticationFailed:
                return ErrorSnapshot(
                    L"Pastral agent authentication failed",
                    L"The local agent identity could not be authenticated. Restart or repair the installation before retrying.");
            case ManagerIpcBridgeStatus::Unhealthy:
                return ErrorSnapshot(
                    L"Pastral agent needs attention",
                    L"The agent reported a privacy-policy or storage-integrity failure. History remains unavailable.");
            case ManagerIpcBridgeStatus::InvalidArgument:
                return ErrorSnapshot(
                    L"Pastral connection configuration is invalid",
                    L"The diagnostic or local data location is not valid for the secure local connection.");
            case ManagerIpcBridgeStatus::AbiMismatch:
                return ErrorSnapshot(
                    L"Pastral bridge versions are incompatible",
                    L"The manager and local IPC bridge use different native interface versions.");
            case ManagerIpcBridgeStatus::InsufficientBuffer:
                return ErrorSnapshot(
                    L"Pastral history changed during refresh",
                    L"The bounded history page changed too quickly to copy safely. Retry the request.");
            case ManagerIpcBridgeStatus::InternalError:
            default:
                return ErrorSnapshot(
                    L"Pastral agent connection failed",
                    L"The manager could not complete the secure local connection check.");
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

        [[nodiscard]] std::wstring RelativeTime(std::int64_t observedAtUnixMicros)
        {
            using namespace std::chrono;
            auto const now = duration_cast<microseconds>(
                system_clock::now().time_since_epoch()).count();
            auto const elapsed = now > observedAtUnixMicros
                ? now - observedAtUnixMicros
                : 0;
            auto const seconds = elapsed / 1'000'000;
            if (seconds < 60)
            {
                return L"Just now";
            }
            auto const minutes = seconds / 60;
            if (minutes < 60)
            {
                return std::to_wstring(minutes) + (minutes == 1 ? L" min ago" : L" min ago");
            }
            auto const hours = minutes / 60;
            if (hours < 24)
            {
                return std::to_wstring(hours) + (hours == 1 ? L" hour ago" : L" hours ago");
            }
            auto const days = hours / 24;
            if (days < 30)
            {
                return std::to_wstring(days) + (days == 1 ? L" day ago" : L" days ago");
            }
            return L"More than a month ago";
        }

        [[nodiscard]] ClipPreviewData MakeClip(
            std::wstring id,
            std::wstring safePreview,
            std::wstring source,
            std::wstring relativeTime,
            std::wstring typeLabel,
            std::wstring profile,
            std::wstring representationSummary,
            std::wstring automationName,
            bool pinned,
            bool unavailable,
            bool previewTruncated = false)
        {
            ClipPreviewData clip;
            clip.id = std::move(id);
            clip.safePreview = std::move(safePreview);
            clip.source = std::move(source);
            clip.relativeTime = std::move(relativeTime);
            clip.typeLabel = std::move(typeLabel);
            clip.profile = std::move(profile);
            clip.representationSummary = std::move(representationSummary);
            clip.automationName = std::move(automationName);
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
            auto const relativeTime = RelativeTime(value.observedAtUnixMicros);
            std::wstring const type = unavailable ? L"Unavailable" : L"Text";
            auto const preview = unavailable
                ? L"Preview unavailable"
                : (value.preview.empty() ? L"Empty text preview" : value.preview);
            auto const representation = unavailable
                ? L"Preview metadata only · Content unavailable"
                : (value.previewTruncated ? L"Text preview · Truncated" : L"Text preview");
            auto automationName = type + L" clip from " + source + L", " + relativeTime;
            if (value.previewTruncated)
            {
                automationName += L", preview truncated";
            }
            return MakeClip(
                FormatUuid(value.eventId),
                preview,
                source,
                relativeTime,
                type,
                L"Ordinary",
                representation,
                automationName,
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
            snapshot.statusTitle = L"Synthetic preview data";
            snapshot.statusDetail =
                L"These bounded examples exercise manager layout and accessibility. They are not clipboard history.";
            snapshot.activeProfile = L"Development";
            snapshot.storageSummary = L"Synthetic examples only · No database or blob access";
            snapshot.query = query;
            snapshot.hasMore = false;
            snapshot.synthetic = true;
            snapshot.clips = {
                MakeClip(
                    L"synthetic-clip-text",
                    L"Build verification passed for the local workspace.",
                    L"Visual Studio Code",
                    L"2 min ago",
                    L"Text",
                    L"Development",
                    L"Unicode text · Plain text",
                    L"Text clip from Visual Studio Code, copied 2 minutes ago",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-code",
                    L"cargo test --locked --workspace --all-targets",
                    L"Windows Terminal",
                    L"8 min ago",
                    L"Code",
                    L"Development",
                    L"Unicode text · Plain text",
                    L"Code clip from Windows Terminal, copied 8 minutes ago",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-url",
                    L"learn.microsoft.com/windows/apps/windows-app-sdk/",
                    L"Microsoft Edge",
                    L"18 min ago",
                    L"Link",
                    L"Ordinary",
                    L"Unicode text · Web link",
                    L"Link from Microsoft Edge, copied 18 minutes ago",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-image",
                    L"Screenshot · 1920 × 1080",
                    L"Snipping Tool",
                    L"34 min ago",
                    L"Image",
                    L"Ordinary",
                    L"PNG · Bitmap",
                    L"Image from Snipping Tool, copied 34 minutes ago",
                    false,
                    false),
                MakeClip(
                    L"synthetic-clip-pinned",
                    L"Release checklist: verify signatures and recovery.",
                    L"Pastral Manager",
                    L"Yesterday",
                    L"Text",
                    L"Development",
                    L"Unicode text · Plain text",
                    L"Pinned text clip from Pastral Manager, copied yesterday",
                    true,
                    false),
                MakeClip(
                    L"synthetic-clip-unavailable",
                    L"Referenced file is no longer available.",
                    L"File Explorer",
                    L"3 days ago",
                    L"File reference",
                    L"Ordinary",
                    L"Reference only",
                    L"Unavailable file reference from File Explorer, copied 3 days ago",
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
                return ErrorSnapshot(
                    L"Pastral connection configuration is invalid",
                    L"The manager could not resolve a safe local data location for the secure connection.");
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
                return ErrorSnapshot(
                    L"Pastral history bridge is unavailable",
                    L"Repair the installation so the manager can load the bounded History interface.");
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
            snapshot.statusDetail = page.hasMore
                ? L"The secure local connection returned the first bounded page of history."
                : L"The secure local connection returned the current bounded history page.";
            snapshot.storageSummary +=
                L" · " + std::to_wstring(snapshot.clips.size()) +
                (snapshot.clips.size() == 1 ? L" item" : L" items");
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
                        snapshot = ErrorSnapshot(
                            L"Pastral agent connection failed",
                            L"The manager could not prepare the secure local connection state.");
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
        snapshot.statusTitle = L"Connecting to local agent";
        snapshot.statusDetail = L"Checking the secure local connection.";
        snapshot.activeProfile = L"Ordinary";
        snapshot.storageSummary = L"Preparing local status";
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
