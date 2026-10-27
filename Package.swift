// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "localwave",
    platforms: [
        .macOS(.v13),
        .iOS(.v16)
    ],
    products: [
        // TUI executable
        .executable(
            name: "localwave-tui",
            targets: ["localwave-tui"]
        ),
        // Shared libraries
        .library(
            name: "LocalWaveDomain",
            targets: ["LocalWaveDomain"]
        ),
        .library(
            name: "LocalWaveData",
            targets: ["LocalWaveData"]
        ),
        .library(
            name: "LocalWaveCore",
            targets: ["LocalWaveCore"]
        ),
        .library(
            name: "LocalWavePlayer",
            targets: ["LocalWavePlayer"]
        )
    ],
    dependencies: [
        // SwiftTUI for terminal interface (local copy for development)
        .package(path: "SwiftTUI"),
        // SQLite.swift - shared with iOS app
        .package(url: "https://github.com/stephencelis/SQLite.swift", from: "0.15.3")
    ],
    targets: [
        // MARK: - Shared Library Targets

        // Domain layer - pure models and protocols, no dependencies
        .target(
            name: "LocalWaveDomain",
            dependencies: [],
            path: "localwave/Sources/Domain"
        ),

        // Core layer - utilities, platform abstractions
        .target(
            name: "LocalWaveCore",
            dependencies: ["LocalWaveDomain"],
            path: "localwave/Sources/Core"
        ),

        // Data layer - repositories, services, SQLite
        .target(
            name: "LocalWaveData",
            dependencies: [
                "LocalWaveDomain",
                "LocalWaveCore",
                .product(name: "SQLite", package: "SQLite.swift")
            ],
            path: "localwave/Sources/Data"
        ),

        // Player layer - ViewModels and player logic
        .target(
            name: "LocalWavePlayer",
            dependencies: [
                "LocalWaveDomain",
                "LocalWaveCore",
                "LocalWaveData"
            ],
            path: "localwave/Sources/Features/Player/ViewModels",
            exclude: ["PlayerViewModel.swift"]
        ),

        // MARK: - TUI Executable Target

        .executableTarget(
            name: "localwave-tui",
            dependencies: [
                "LocalWaveDomain",
                "LocalWaveCore",
                "LocalWaveData",
                "LocalWavePlayer",
                "SwiftTUI"
            ],
            path: "localwave-tui/Sources",
            swiftSettings: [
                .unsafeFlags(["-parse-as-library"])
            ]
        )
    ]
)
