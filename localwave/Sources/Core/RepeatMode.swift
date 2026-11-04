//
//  RepeatMode.swift
//  localwave
//
//  Created by Claude Code on 04.10.2025.
//

import Foundation
import SwiftUI

/// Platform-agnostic repeat mode enumeration
public enum RepeatMode: Int, CaseIterable, Sendable {
    case none, all, one

    public mutating func toggle() {
        self = Self(rawValue: (rawValue + 1) % Self.allCases.count)!
    }

    public var displayName: String {
        switch self {
        case .none: return "Off"
        case .all:  return "All"
        case .one:  return "One"
        }
    }

    public var symbol: String {
        switch self {
        case .none: return "○"
        case .all:  return "↻"
        case .one:  return "1↻"
        }
    }
    
    public var iconName: String {
        switch self {
        case .none: return "repeat.circle"
        case .all:  return "repeat.circle.fill"
        case .one:  return "repeat.1.circle.fill"
        }
    }
    
    public var color: Color {
        switch self {
        case .none: return .white
        case .all:  return .yellow
        case .one:  return .yellow
        }
    }
}
