#if os(iOS)
import LocalWaveData
import LocalWaveCore
import LocalWaveDomain
//
//  CustomTabView.swift
//  localwave
//
//  Created by Oleg Pustovit on 20.05.2025.
//

import AVFoundation
import Combine
import MediaPlayer
import os
import SwiftUI

struct CustomTabView: View {
    @Binding var selection: Int
    let tabs: [TabItem]

    init(selection: Binding<Int>, @TabItemBuilder content: () -> [TabItem]) {
        _selection = selection
        tabs = content()
    }

    var body: some View {
        ZStack {
            ForEach(tabs.indices, id: \.self) { index in
                tabs[index].content
                    .opacity(selection == index ? 1 : 0)
                    .animation(nil, value: selection)
            }
        }
        .overlay(
            // Your custom tab bar remains the same…
            HStack {
                ForEach(tabs.indices, id: \.self) { index in
                    Button(action: {
                        selection = index
                    }) {
                        VStack(spacing: 4) {
                            Image(systemName: tabs[index].systemImage)
                                .font(.system(size: 22, weight: .semibold))
                            Text(tabs[index].label)
                                .font(.caption)
                        }
                        .foregroundColor(selection == index ? .accentColor : .gray)
                        .frame(maxWidth: .infinity)
                    }
                }
            }
            .padding(.top)
            .frame(height: 60)
            .background(.thinMaterial),
            alignment: .bottom
        )
    }

    struct TabViewContainer: View, TabViewBuilder {
        let tabs: [TabItem]

        var body: some View {
            EmptyView()
        }
    }

    static func buildBlock(_ components: TabItem...) -> TabViewContainer {
        return TabViewContainer(tabs: components)
    }
}

#endif
