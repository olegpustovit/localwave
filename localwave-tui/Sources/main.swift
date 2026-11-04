import Foundation
import LocalWaveCore

@main
struct LocalWaveTUI {
    static func main() async {
        // Configure TUI logging mode (file-based, not stdout)
        configureLogging(useTUILogger: true)

        // Initialize file logger (must be first to capture all logs)
        let logPath = FileLogger.shared.getLogFilePath()
        fputs("📝 Logs: \(logPath)\n", stderr)

        do {
            let app = try TUIApp()
            try await app.start()
        } catch {
            fputs("❌ Error: \(error.localizedDescription)\n", stderr)
            exit(1)
        }
    }
}
