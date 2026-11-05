#if os(iOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  TabItem.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct TabItem: Identifiable {
    let id = UUID()
    let label: String
    let systemImage: String
    let content: AnyView
    let tag: Int

    init<Content: View>(
        label: String, systemImage: String, tag: Int, @ViewBuilder content: () -> Content
    ) {
        self.label = label
        self.systemImage = systemImage
        self.tag = tag
        self.content = AnyView(content())
    }
}

#endif
