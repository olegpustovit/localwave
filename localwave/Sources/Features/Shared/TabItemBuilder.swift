#if os(iOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  TabItemBuilder.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

@resultBuilder
struct TabItemBuilder {
    static func buildBlock(_ components: TabItem...) -> [TabItem] {
        components
    }
}

#endif
