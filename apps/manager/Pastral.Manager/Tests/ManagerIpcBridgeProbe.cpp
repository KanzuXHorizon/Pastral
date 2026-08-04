#include "../Services/ManagerIpcBridge.h"

#include <cstdint>
#include <iostream>
#include <string>
#include <string_view>

using Pastral::Manager::Services::ManagerIpcBridge;
using Pastral::Manager::Services::ManagerIpcBridgeStatus;

int wmain(int argc, wchar_t** argv)
{
    if (argc == 2 && std::wstring_view{ argv[1] } == L"--abi")
    {
        if (!ManagerIpcBridge::IsAvailable() || !ManagerIpcBridge::IsReadAvailable())
        {
            return 1;
        }
        std::wcout << L"manager-ipc-abi=ok\n";
        std::wcout << L"manager-ipc-read-abi=ok\n";
        return 0;
    }

    if (argc == 4 &&
        std::wstring_view{ argv[1] } == L"--health" &&
        std::wstring_view{ argv[2] } == L"--data-root" &&
        argv[3][0] != L'\0')
    {
        auto const result = ManagerIpcBridge::QueryHealth(argv[3], 2000);
        std::wcout << L"manager-ipc-probe=ok\n";
        std::wcout << L"status=" << static_cast<std::uint32_t>(result.status) << L'\n';
        std::wcout << L"storage-schema=" << result.storageSchemaVersion << L'\n';
        std::wcout << L"capture-enabled=" << (result.captureEnabled ? 1 : 0) << L'\n';
        std::wcout << L"privacy-policy-ok=" << (result.privacyPolicyOk ? 1 : 0) << L'\n';
        std::wcout << L"storage-integrity-ok=" << (result.storageIntegrityOk ? 1 : 0) << L'\n';
        std::wcout << L"server-pid=" << result.serverProcessId << L'\n';
        std::wcout << L"session-id=" << result.sessionId << L'\n';
        std::wcout << L"connect-us=" << result.connectMicroseconds << L'\n';
        std::wcout << L"handshake-us=" << result.handshakeMicroseconds << L'\n';
        std::wcout << L"health-us=" << result.healthMicroseconds << L'\n';
        return result.status == ManagerIpcBridgeStatus::Connected ? 0 : 1;
    }

    if (argc == 4 &&
        std::wstring_view{ argv[1] } == L"--read" &&
        std::wstring_view{ argv[2] } == L"--data-root" &&
        argv[3][0] != L'\0')
    {
        auto const history = ManagerIpcBridge::QueryHistory(argv[3], 2000, 50);
        auto const search = ManagerIpcBridge::QuerySearch(argv[3], 2000, L"probe", 50);
        std::wcout << L"manager-ipc-read-probe=ok\n";
        std::wcout << L"history-status=" << static_cast<std::uint32_t>(history.status) << L'\n';
        std::wcout << L"history-count=" << history.items.size() << L'\n';
        std::wcout << L"history-has-more=" << (history.hasMore ? 1 : 0) << L'\n';
        std::wcout << L"search-status=" << static_cast<std::uint32_t>(search.status) << L'\n';
        std::wcout << L"search-count=" << search.items.size() << L'\n';
        std::wcout << L"search-has-more=" << (search.hasMore ? 1 : 0) << L'\n';
        return history.status == ManagerIpcBridgeStatus::Connected &&
                       search.status == ManagerIpcBridgeStatus::Connected &&
                       history.items.empty() &&
                       search.items.empty() &&
                       !history.hasMore &&
                       !search.hasMore
            ? 0
            : 1;
    }

    return 2;
}
