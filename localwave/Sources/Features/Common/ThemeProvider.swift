import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  ThemeProvider.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//
import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

func Oxanium(_ size: CGFloat = 16) -> Font {
    return Font.custom("Oxanium", size: size)
}

struct ThemeProvider: ViewModifier {
    func body(content: Content) -> some View {
        content
            // .environment(\.font, .system(size: 18, weight: .medium))  // Global font
            .font(Oxanium())
            .accentColor(.purple)
            .environment(\.colorScheme, .dark)
            .preferredColorScheme(.dark) // Force dark mode
    }
}

extension View {
    func applyTheme() -> some View {
        modifier(ThemeProvider())
    }
}
