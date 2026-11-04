import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  TabState.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

class TabState: ObservableObject {
    @Published var selectedTab: Int = 0
}
