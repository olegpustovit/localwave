//
//  FileLogger.swift
//  localwave
//
//  Created by Claude Code on 05.10.2025.
//

import Foundation
import os

// MARK: - Logger Protocol

/// Universal logging protocol that works across iOS and TUI
public protocol AppLogger {
    func debug(_ message: String)
    func info(_ message: String)
    func warning(_ message: String)
    func error(_ message: String)
    func fault(_ message: String)
}

// MARK: - iOS Logger (os.Logger wrapper)

/// iOS logger that wraps os.Logger for console output
public struct IOSLogger: AppLogger {
    private let logger: Logger

    public init(subsystem: String, category: String) {
        self.logger = Logger(subsystem: subsystem, category: category)
    }

    public func debug(_ message: String) {
        logger.debug("\(message)")
    }

    public func info(_ message: String) {
        logger.info("\(message)")
    }

    public func warning(_ message: String) {
        logger.warning("\(message)")
    }

    public func error(_ message: String) {
        logger.error("\(message)")
    }

    public func fault(_ message: String) {
        logger.fault("\(message)")
    }
}

// MARK: - File Logger Backend

/// File-based logger backend for TUI applications
/// Redirects all log output to a file to avoid breaking TUI display
public class FileLogger {
    public static let shared = FileLogger()

    private let logFileURL: URL
    private let fileHandle: FileHandle?
    private let queue = DispatchQueue(label: "com.snowbear.localwave.filelogger", qos: .utility)

    private init() {
        // Create logs directory
        let logsDir: URL

        #if os(macOS)
        if let xdgDataHome = ProcessInfo.processInfo.environment["XDG_DATA_HOME"] {
            logsDir = URL(fileURLWithPath: "\(xdgDataHome)/localwave/logs")
        } else {
            let homeDir = FileManager.default.homeDirectoryForCurrentUser
            logsDir = homeDir.appendingPathComponent("Library/Application Support/localwave/logs")
        }
        #else
        let homeDir = FileManager.default.homeDirectoryForCurrentUser
        logsDir = homeDir.appendingPathComponent(".local/share/localwave/logs")
        #endif

        // Create directory if needed
        try? FileManager.default.createDirectory(at: logsDir, withIntermediateDirectories: true)

        // Create log file with timestamp
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd_HH-mm-ss"
        let timestamp = dateFormatter.string(from: Date())
        logFileURL = logsDir.appendingPathComponent("tui-\(timestamp).log")

        // Create or open file
        if !FileManager.default.fileExists(atPath: logFileURL.path) {
            FileManager.default.createFile(atPath: logFileURL.path, contents: nil)
        }

        fileHandle = try? FileHandle(forWritingTo: logFileURL)

        // Write header
        let header = """
        ================================================================================
        LocalWave TUI Log
        Started: \(Date())
        Log file: \(logFileURL.path)
        ================================================================================

        """
        writeToFile(header)

        print("📝 Logging to: \(logFileURL.path)", to: &standardError)
    }

    deinit {
        try? fileHandle?.close()
    }

    /// Write a log message to the file
    public func log(_ message: String, level: String = "INFO", file: String = #file, function: String = #function, line: Int = #line) {
        let timestamp = ISO8601DateFormatter().string(from: Date())
        let fileName = (file as NSString).lastPathComponent
        let logMessage = "[\(timestamp)] [\(level)] [\(fileName):\(line)] \(function) - \(message)\n"
        writeToFile(logMessage)
    }

    private func writeToFile(_ message: String) {
        queue.async { [weak self] in
            guard let data = message.data(using: .utf8) else { return }
            self?.fileHandle?.write(data)
        }
    }

    /// Get the current log file path
    public func getLogFilePath() -> String {
        return logFileURL.path
    }
}

/// Custom TextOutputStream for stderr
private struct StandardError: TextOutputStream {
    mutating func write(_ string: String) {
        fputs(string, stderr)
    }
}

private var standardError = StandardError()

// MARK: - TUI Logger (File-based)

/// TUI logger that writes to file instead of stdout
public struct TUILogger: AppLogger {
    private let subsystem: String
    private let category: String

    public init(subsystem: String, category: String) {
        self.subsystem = subsystem
        self.category = category
    }

    public func debug(_ message: String) {
        FileLogger.shared.log("[\(subsystem)][\(category)] \(message)", level: "DEBUG")
    }

    public func info(_ message: String) {
        FileLogger.shared.log("[\(subsystem)][\(category)] \(message)", level: "INFO")
    }

    public func warning(_ message: String) {
        FileLogger.shared.log("[\(subsystem)][\(category)] \(message)", level: "WARNING")
    }

    public func error(_ message: String) {
        FileLogger.shared.log("[\(subsystem)][\(category)] \(message)", level: "ERROR")
    }

    public func fault(_ message: String) {
        FileLogger.shared.log("[\(subsystem)][\(category)] \(message)", level: "FAULT")
    }
}

// MARK: - Platform-Agnostic Logger Factory

/// Global flag to determine logger type (set by main app)
private var _useTUILogger = false

/// Configure logging mode for the application
/// - Parameter useTUILogger: true for file-based logging (TUI), false for console logging (iOS)
public func configureLogging(useTUILogger: Bool) {
    _useTUILogger = useTUILogger
}

/// Creates a platform-appropriate logger
/// - Returns: TUILogger for TUI apps, IOSLogger for iOS apps
public func createLogger(subsystem: String, category: String) -> AppLogger {
    if _useTUILogger {
        return TUILogger(subsystem: subsystem, category: category)
    } else {
        return IOSLogger(subsystem: subsystem, category: category)
    }
}
