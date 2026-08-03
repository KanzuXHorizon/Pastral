#include "pch.h"
#include "ManagerDataProvider.h"

namespace Pastral::Manager::Presentation
{
    namespace
    {
        class ManagerDataProvider final : public IManagerDataProvider
        {
        public:
            ManagerSnapshot LoadSnapshot() const override
            {
                ManagerSnapshot snapshot;

#if defined(_DEBUG)
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
#else
                snapshot.connection = ConnectionState::Disconnected;
                snapshot.statusTitle = L"Pastral agent is not connected";
                snapshot.statusDetail =
                    L"History and paste actions will become available after the versioned local IPC service is implemented.";
                snapshot.activeProfile = L"Ordinary";
                snapshot.storageSummary = L"Unavailable until the local agent is connected";
                snapshot.clips = {};
                snapshot.synthetic = false;
#endif

                return snapshot;
            }

        private:
            static ClipPreviewData MakeClip(
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
        };
    }

    std::shared_ptr<IManagerDataProvider> CreateManagerDataProvider()
    {
        return std::make_shared<ManagerDataProvider>();
    }
}
