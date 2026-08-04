#include "pch.h"
#include "ManagerDataProvider.h"
#include "ManagerIpcBridge.h"

#include <condition_variable>
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
        constexpr DWORD MaximumEnvironmentCharacters = 32768;

        struct PendingRequest final
        {
            std::uint64_t generation{};
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
            snapshot.synthetic = false;
            return snapshot;
        }

        [[nodiscard]] ManagerSnapshot LiveSnapshot(
            Services::ManagerIpcBridgeHealth const& health)
        {
            ManagerSnapshot snapshot;
            snapshot.activeProfile = L"Ordinary";
            snapshot.clips.clear();
            snapshot.synthetic = false;

            using Services::ManagerIpcBridgeStatus;
            switch (health.status)
            {
            case ManagerIpcBridgeStatus::Connected:
                snapshot.connection = ConnectionState::Connected;
                snapshot.statusTitle = L"Pastral agent is connected";
                snapshot.statusDetail =
                    L"The authenticated local Health endpoint passed privacy and storage integrity checks.";
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
                    L"The bounded local Health request timed out. Retry after checking the agent.";
                snapshot.storageSummary = L"Unavailable because the local Health request timed out";
                break;
            case ManagerIpcBridgeStatus::ProtocolMismatch:
                snapshot.connection = ConnectionState::ProtocolMismatch;
                snapshot.statusTitle = L"Pastral versions are incompatible";
                snapshot.statusDetail =
                    L"The manager and local agent could not negotiate a compatible Health protocol.";
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
                    L"The diagnostic or local data location is not valid for the manager Health connection.");
            case ManagerIpcBridgeStatus::AbiMismatch:
                return ErrorSnapshot(
                    L"Pastral bridge versions are incompatible",
                    L"The manager and local IPC bridge use different native interface versions.");
            case ManagerIpcBridgeStatus::InternalError:
            default:
                return ErrorSnapshot(
                    L"Pastral agent connection failed",
                    L"The manager could not complete the local authenticated Health check.");
            }
            return snapshot;
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
            bool unavailable)
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
            return clip;
        }

        [[nodiscard]] ManagerSnapshot SyntheticSnapshot()
        {
            ManagerSnapshot snapshot;
            snapshot.connection = ConnectionState::Connected;
            snapshot.statusTitle = L"Synthetic preview data";
            snapshot.statusDetail =
                L"These bounded examples exercise manager layout and accessibility. They are not clipboard history.";
            snapshot.activeProfile = L"Development";
            snapshot.storageSummary = L"Synthetic examples only · No database or blob access";
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
            return snapshot;
        }

        [[nodiscard]] ManagerSnapshot BuildSnapshot()
        {
#if defined(_DEBUG)
            auto const diagnosticFlag = ReadEnvironment(DiagnosticFlagName);
            if (!diagnosticFlag.has_value())
            {
                return SyntheticSnapshot();
            }
#endif
            auto const root = ResolveDataRoot();
            if (root.mode == DataRootMode::Invalid)
            {
                return ErrorSnapshot(
                    L"Pastral connection configuration is invalid",
                    L"The manager could not resolve a safe local data location for the Health connection.");
            }
            return LiveSnapshot(
                Services::ManagerIpcBridge::QueryHealth(
                    root.path.native(),
                    HealthTimeoutMilliseconds));
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
                    m_pending = PendingRequest{ m_generation, std::move(completion) };
                }
                m_condition.notify_one();
            }

        private:
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
                        snapshot = BuildSnapshot();
                    }
                    catch (...)
                    {
                        snapshot = ErrorSnapshot(
                            L"Pastral agent connection failed",
                            L"The manager could not prepare the local Health connection state.");
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
        snapshot.statusTitle = L"Connecting to Pastral agent";
        snapshot.statusDetail = L"Verifying the authenticated local Health endpoint.";
        snapshot.activeProfile = L"Ordinary";
        snapshot.storageSummary = L"Checking local agent integrity";
        snapshot.clips.clear();
        snapshot.synthetic = false;
        return snapshot;
    }

    std::shared_ptr<IManagerDataProvider> CreateManagerDataProvider()
    {
        return std::make_shared<ManagerDataProvider>();
    }
}
