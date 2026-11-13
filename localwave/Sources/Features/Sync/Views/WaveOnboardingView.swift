#if os(iOS) || os(macOS)
import SwiftUI
import UniformTypeIdentifiers
import LocalWaveCore

struct WaveOnboardingView: View {
    let dependencies: DependencyContainer
    let onImported: () -> Void
    let onExplore: () -> Void
    @EnvironmentObject private var playerVM: PlayerViewModel
    @State private var folderPicker = false
    @State private var audioPicker = false
    @State private var driveExpanded = false
    @State private var adding = false
    @State private var error: String?
    @State private var demoAvailable = false
    @State private var checkingDemo = true
    @State private var songCount = 0

    var body: some View {
        layout
        .background(LWTheme.paper).foregroundStyle(LWTheme.ink).buttonStyle(.plain)
        .accessibilityElement(children: .contain).accessibilityAddTraits(.isModal)
        .task { await checkDemoAvailability() }
        .fileImporter(isPresented: $folderPicker, allowedContentTypes: [.folder], allowsMultipleSelection: true) { importFiles($0, play: false) }
        // Two fileImporters on one view: SwiftUI honours only the last, so the audio one lives on a background node.
        .background(Color.clear.fileImporter(isPresented: $audioPicker, allowedContentTypes: [.audio], allowsMultipleSelection: true) { importFiles($0, play: true) })
        .alert("Unable to add music", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
            Button("OK") { error = nil }
        } message: { Text(error ?? "") }
    }

    @ViewBuilder private var layout: some View {
        #if os(macOS)
        macLayout
        #else
        phoneLayout
        #endif
    }

    #if os(macOS)
    /// Two columns once the window is wide enough; one centred column below that. Content is capped so it never stretches across a big display.
    private var macLayout: some View {
        GeometryReader { bounds in
            let wide = bounds.size.width >= 760
            ScrollView {
                Group {
                    if wide {
                        HStack(alignment: .center, spacing: 72) {
                            macIntro.frame(maxWidth: .infinity, alignment: .leading)
                            macActions.frame(maxWidth: 400)
                        }
                    } else {
                        VStack(alignment: .leading, spacing: 40) { macIntro; macActions }.frame(maxWidth: 480)
                    }
                }
                .padding(.horizontal, 40).padding(.vertical, 40)
                .frame(maxWidth: 1040)
                .frame(maxWidth: .infinity, minHeight: bounds.size.height, alignment: .center)
            }.scrollIndicators(.automatic)
        }
    }

    private var macIntro: some View {
        VStack(alignment: .leading, spacing: 0) {
            MatrixText(text: String(format: "%02d", songCount), height: 96)
            Text("songs so far").font(LWTheme.text(15)).foregroundStyle(LWTheme.secondary).padding(.top, 8)
            Text("Point it at a folder.\nIt stays in sync.")
                .font(LWTheme.text(44).weight(.medium)).tracking(-1.5)
                .fixedSize(horizontal: false, vertical: true).padding(.top, 32)
            Text("Your files stay where they are. Nothing is uploaded.")
                .font(LWTheme.text(16)).foregroundStyle(LWTheme.secondary).padding(.top, 12)
        }
    }

    private var macActions: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(spacing: 0) {
                Rectangle().fill(LWTheme.line).frame(height: 1)
                step("1", "Choose a folder, or drop one in this window")
                step("2", "LocalWave indexes it in the background")
                step("3", "Play at once; iCloud songs download on play")
            }
            Button { folderPicker = true } label: {
                HStack(spacing: 8) {
                    LWIcon("folder", size: 20)
                    Text(adding ? "Adding music…" : "Choose a folder…").lwLineBox(15, 21)
                }.font(LWTheme.text(15).weight(.medium)).frame(maxWidth: .infinity).frame(height: 48)
                    .background(LWTheme.ink).foregroundStyle(LWTheme.onInk)
            }.buttonStyle(LWKeyStyle()).disabled(adding).keyboardShortcut(.defaultAction).padding(.top, 28)
            Button { audioPicker = true } label: { underlined("Pick some files") }.disabled(adding).padding(.top, 8)
            Text("iCloud Drive, external drives, network shares: anything Finder can open.")
                .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).padding(.top, 12)
        }
    }
    #endif

    private var phoneLayout: some View {
        GeometryReader { bounds in
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    HStack(alignment: .top) {
                        Text("LocalWave").font(LWTheme.text(13)).lwLineBox(13, 18.2)
                        Spacer()
                        Text("v2").font(LWTheme.data(12)).lwLineBox(12, 16.8, data: true)
                    }.foregroundStyle(LWTheme.secondary).frame(minHeight: 18.2)
                    MatrixText(text: String(format: "%02d", songCount), height: 79).padding(.top, 28)
                    Text("songs so far").font(LWTheme.text(15)).foregroundStyle(LWTheme.secondary).lwLineBox(15, 21)
                        .frame(minHeight: 21).padding(.top, 8)
                    heading.padding(.top, 28)
                    Text("Your files stay where they are. Nothing is uploaded.")
                        .font(LWTheme.text(15)).foregroundStyle(LWTheme.secondary)
                        .lwLineBox(15, 21).fixedSize(horizontal: false, vertical: true).padding(.top, 8)
                    VStack(spacing: 0) {
                        Rectangle().fill(LWTheme.line).frame(height: 1)
                        step("1", "Choose a folder in Files")
                        step("2", "LocalWave indexes it in the background")
                        step("3", "Play at once; cloud songs fetch on play")
                    }.padding(.top, 28)
                    Spacer(minLength: 0)
                    footer
                }
                .frame(minHeight: max(0, bounds.size.height - 50), alignment: .top)
                .padding(.horizontal, 16).padding(.top, 16).padding(.bottom, 34)
            }.scrollIndicators(.hidden)
        }
    }

    private var heading: some View {
        Text("Point it at a folder.\nIt stays in sync.")
            .font(LWTheme.text(32).weight(.medium)).tracking(-1.12)
            .lwLineBox(32, 36.8).fixedSize(horizontal: false, vertical: true)
    }

    private var footer: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("iCloud Drive, Google Drive, On My iPhone, USB, SMB — anything in Files.")
                .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                .lwLineBox(13, 18.2).fixedSize(horizontal: false, vertical: true).padding(.bottom, 12)
            Button { folderPicker = true } label: {
                HStack(spacing: 8) {
                    LWIcon("folder", size: 20)
                    Text(adding ? "Adding music…" : "Choose a folder").lwLineBox(15, 21)
                }.font(LWTheme.text(15).weight(.medium)).frame(maxWidth: .infinity).frame(height: 52)
                    .background(LWTheme.ink).foregroundStyle(LWTheme.onInk)
            }.buttonStyle(LWKeyStyle()).disabled(adding)
            HStack {
                Button { audioPicker = true } label: { underlined("Pick some files") }.disabled(adding)
                Spacer(minLength: 12)
                Button(action: exploreOrTryDemo) { underlined(demoAvailable ? "Try the demo library" : "Explore the library") }
                    .disabled(adding || checkingDemo)
            }.padding(.top, 4)
            VStack(alignment: .leading, spacing: 0) {
                Button { driveExpanded.toggle() } label: {
                    HStack(spacing: 0) {
                        WaveDisclosureMarker(expanded: driveExpanded)
                        Text("Google Drive not in the list?").font(LWTheme.text(13)).lwLineBox(13, 19.5)
                        Spacer(minLength: 0)
                    }.frame(minHeight: 19.5)
                }.accessibilityValue(driveExpanded ? "Expanded" : "Collapsed")
                if driveExpanded {
                    Text("Install the Google Drive app, then in Files tap Browse → ⋯ → Edit and turn on Google Drive.")
                        .font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary)
                        .lwLineBox(13, 19.5).fixedSize(horizontal: false, vertical: true).padding(.top, 8)
                }
            }.padding(15).overlay { Rectangle().strokeBorder(LWTheme.line) }.padding(.top, 4)
        }
    }

    private func step(_ number: String, _ text: String) -> some View {
        HStack(alignment: .top, spacing: 14) {
            Text(number).font(LWTheme.text(13)).foregroundStyle(LWTheme.secondary).lwLineBox(13, 18.2).frame(width: 12, alignment: .leading)
            Text(text).font(LWTheme.text(15)).lwLineBox(15, 21).frame(maxWidth: .infinity, minHeight: 21, alignment: .leading)
        }.padding(.vertical, 12).padding(.bottom, 1)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1) }
    }

    private func underlined(_ text: String) -> some View {
        Text(text).font(LWTheme.text(15)).lwLineBox(15, 21).frame(minHeight: 21)
            .overlay(alignment: .bottom) { Rectangle().fill(LWTheme.line).frame(height: 1).offset(y: -1) }
            .frame(minHeight: 44)
    }

    private func checkDemoAvailability() async {
        #if DEBUG
        if LWReferenceFixture.enabled {
            // The reference onboarding represents a fresh library before its
            // bundled demonstration is entered. Keep that exact fixture state.
            demoAvailable = true
            checkingDemo = false
            return
        }
        #endif
        defer { checkingDemo = false }
        do {
            let count = try await dependencies.songRepository.totalSongCount(query: "")
            guard !Task.isCancelled else { return }
            songCount = count
            demoAvailable = count == 0
        } catch { self.error = error.localizedDescription }
    }

    private func exploreOrTryDemo() {
        #if DEBUG
        if LWReferenceFixture.enabled { onExplore(); return }
        #endif
        guard !adding else { return }
        guard demoAvailable else { onExplore(); return }
        adding = true
        Task {
            defer { adding = false }
            do {
                _ = try await LWDemoLibrary.installIfEmpty(dependencies: dependencies)
                onExplore()
            } catch {
                self.error = error.localizedDescription
                // A partial import remains a normal source. Do not reimport
                // over its rows; Explore opens it and Folders offers rescan.
                let count = try? await dependencies.songRepository.totalSongCount(query: "")
                if let count { songCount = count; demoAvailable = count == 0 }
            }
        }
    }

    private func importFiles(_ result: Result<[URL], Error>, play: Bool) {
        Task {
            adding = true
            defer { adding = false }
            do {
                let songs = try await dependencies.addMusicURLs(result.get())
                if songs.isEmpty { throw CustomError.genericError("No supported audio files were found in the selection.") }
                if play, let first = songs.first {
                    playerVM.configureQueue(songs: songs, startIndex: 0, context: .selection("Selected files"))
                    playerVM.playSong(first)
                }
                onImported()
            } catch { self.error = error.localizedDescription }
        }
    }
}

/// The browser's disclosure marker occupies a 13⅓px inline slot. Its filled
/// triangle is 7×8⅓px closed, 8⅓×7px open, aligned with the summary's text baseline.
private struct WaveDisclosureMarker: View {
    let expanded: Bool
    @ScaledMetric(relativeTo: .caption) private var scale: CGFloat = 1

    var body: some View {
        Path { path in
            let width: CGFloat = expanded ? 25 / 3 : 7
            let height: CGFloat = expanded ? 7 : 25 / 3
            let top = 14 - height
            if expanded {
                path.move(to: CGPoint(x: 0, y: top))
                path.addLine(to: CGPoint(x: width, y: top))
                path.addLine(to: CGPoint(x: width / 2, y: 14))
            } else {
                path.move(to: CGPoint(x: 0, y: top))
                path.addLine(to: CGPoint(x: width, y: top + height / 2))
                path.addLine(to: CGPoint(x: 0, y: 14))
            }
            path.closeSubpath()
        }
        .fill(LWTheme.ink).frame(width: 40 / 3, height: 19.5)
        .scaleEffect(scale, anchor: .leading)
        .frame(width: 40 / 3 * scale, height: 19.5 * scale, alignment: .leading)
        .accessibilityHidden(true)
    }
}

#endif
