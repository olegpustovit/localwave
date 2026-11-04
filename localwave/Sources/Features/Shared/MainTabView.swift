import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  MainTabView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct MainTabView: View {
    @EnvironmentObject private var dependencies: DependencyContainer
    @EnvironmentObject private var tabState: TabState
    @EnvironmentObject private var playerVM: PlayerViewModel
    @State private var isPlayerPresented = false
    @StateObject private var libraryNavigation = LibraryNavigation()

    var body: some View {
        ZStack(alignment: .bottom) {
            CustomTabView(selection: $tabState.selectedTab) {
                TabItem(label: "Library", systemImage: "books.vertical", tag: 0) {
                    LibraryView(dependencies: dependencies)
                        .environmentObject(libraryNavigation)
                }
                TabItem(label: "Sync", systemImage: "icloud.and.arrow.down", tag: 1) {
                    SyncView(dependencies: dependencies)
                }
            }
            .environmentObject(tabState)
            .accentColor(.cyan)

            MiniPlayerView {
                isPlayerPresented = true
            }
            .padding(.bottom, 60)
        }
        .fullScreenCover(isPresented: $isPlayerPresented) {
            PlayerView().environmentObject(playerVM)
        }
    }
}
