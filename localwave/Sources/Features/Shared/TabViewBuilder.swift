import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  TabViewBuilder.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

protocol TabViewBuilder {
    var tabs: [TabItem] { get }
}
