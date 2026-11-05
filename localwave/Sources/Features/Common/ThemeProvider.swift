#if os(iOS)
import LocalWaveCore
import SwiftUI
import ImageIO
import LocalWaveDomain

enum LWTheme {
    private static func color(_ light: UInt32, _ dark: UInt32) -> Color {
        Color(UIColor { traits in
            let hex = traits.userInterfaceStyle == .dark ? dark : light
            return UIColor(red: CGFloat((hex >> 16) & 255) / 255, green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: 1)
        })
    }
    static let paper = color(0xF3F3F3, 0x101010)
    static let panel = color(0xE6E6E6, 0x1A1A1A)
    static let panel2 = color(0xD3D3D3, 0x2A2A2A)
    static let key = color(0xFBFBFB, 0x1F1F1F)
    static let key2 = color(0xDADADA, 0x2E2E2E)
    static let onInk = color(0xF3F3F3, 0x0F0F0F)
    static let wave = color(0xCFCFCF, 0x363636)
    static let danger = Color(red: 210 / 255, green: 26 / 255, blue: 26 / 255)
    static let ink = color(0x111111, 0xEDEDED)
    static let secondary = color(0x585858, 0x9B9B9B)
    static let tertiary = color(0xA0A0A0, 0x5A5A5A)
    static let line = color(0xDADADA, 0x262626)
    static var signal: Color {
        switch LocalWavePreferences.current.string(forKey: "lwSignal") {
        case "red": return Color(red: 227 / 255, green: 36 / 255, blue: 27 / 255)
        case "green": return Color(red: 0, green: 168 / 255, blue: 107 / 255)
        case "mono": return ink
        default: return color(0x2340FF, 0x6F7DFF)
        }
    }
    static func text(_ size: CGFloat = 15) -> Font { .custom("HostGrotesk-Regular", size: size, relativeTo: .body) }
    static func data(_ size: CGFloat = 12) -> Font { .custom("GeistMono-Regular", size: size, relativeTo: .caption) }
    static let motion = LWMotion.out()
}

enum LWMotion {
    static let instant = 0.08
    static let quick = 0.14
    static let base = 0.24
    static let slow = 0.42
    static let ambient = 1.6
    static let stagger = 0.018
    static func out(_ duration: Double = slow) -> Animation { .timingCurve(0.16, 1, 0.3, 1, duration: duration) }
    static func `in`(_ duration: Double = base) -> Animation { .timingCurve(0.7, 0, 0.84, 0, duration: duration) }
    static func inOut(_ duration: Double = base) -> Animation { .timingCurve(0.65, 0, 0.35, 1, duration: duration) }
    static func springAnimation(_ duration: Double = base) -> Animation { Animation(LWCSSSpring(duration: duration)) }
    static func steps(_ count: Int, duration: Double) -> Animation { Animation(LWCSSSteps(count: count, duration: duration)) }
    static func steppedProgress(_ elapsed: Double, duration: Double, count: Int) -> Double {
        // Date subtraction can fall a fraction of a microsecond below a scheduled boundary.
        max(0, min(1, floor((elapsed + 0.000001) / duration * Double(count)) / Double(count)))
    }
    static func outProgress(_ progress: Double) -> Double {
        let target = max(0, min(1, progress))
        var lower = 0.0, upper = 1.0
        for _ in 0..<24 {
            let t = (lower + upper) / 2
            let x = 3 * (1 - t) * (1 - t) * t * 0.16 + 3 * (1 - t) * t * t * 0.3 + t * t * t
            if x < target { lower = t } else { upper = t }
        }
        let t = (lower + upper) / 2
        return 3 * (1 - t) * (1 - t) * t + 3 * (1 - t) * t * t + t * t * t
    }
    static func springProgress(_ progress: Double) -> Double {
        let points: [(Double, Double)] = [(0, 0), (0.0105, 0.009), (0.021, 0.035), (0.044, 0.141), (0.129, 0.723), (0.167, 0.938), (0.205, 1.017), (0.245, 1.043), (0.284, 1.035), (0.377, 1.01), (0.472, 0.997), (1, 1)]
        let p = max(0, min(1, progress))
        for index in 1..<points.count where p <= points[index].0 {
            let a = points[index - 1], b = points[index]
            return a.1 + (b.1 - a.1) * (p - a.0) / (b.0 - a.0)
        }
        return 1
    }
}

private struct LWCSSSpring: CustomAnimation {
    let duration: Double
    func animate<V: VectorArithmetic>(value: V, time: TimeInterval, context: inout AnimationContext<V>) -> V? {
        guard time < duration else { return nil }
        return value.scaled(by: LWMotion.springProgress(time / duration))
    }
}

private struct LWCSSSteps: CustomAnimation {
    let count: Int
    let duration: Double
    func animate<V: VectorArithmetic>(value: V, time: TimeInterval, context: inout AnimationContext<V>) -> V? {
        guard time < duration else { return nil }
        return value.scaled(by: LWMotion.steppedProgress(time, duration: duration, count: count))
    }
}

struct LWPop<Trigger: Equatable>: ViewModifier {
    let trigger: Trigger
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var changedAt = Date.distantPast
    func body(content: Content) -> some View {
        TimelineView(.explicit(reduceMotion ? [] : (0...15).map { changedAt.addingTimeInterval(Double($0) * LWMotion.base / 15) })) { timeline in
            let age = timeline.date.timeIntervalSince(changedAt)
            content.scaleEffect(reduceMotion || age >= LWMotion.base ? 1 : 0.3 + 0.7 * LWMotion.springProgress(age / LWMotion.base))
        }.onChange(of: trigger) { _, _ in changedAt = Date() }
    }
}

struct LWAppear: ViewModifier {
    let translation: CGSize
    var duration = LWMotion.slow
    var enabled = true
    @Environment(\.lwReduceMotion) private var reduceMotion
    @State private var visible = false

    func body(content: Content) -> some View {
        let shown = visible || reduceMotion || !enabled
        content.opacity(shown ? 1 : 0)
            .offset(x: shown ? 0 : translation.width, y: shown ? 0 : translation.height)
            .animation(reduceMotion ? nil : LWMotion.out(duration), value: visible)
            .task {
                await Task.yield()
                guard !Task.isCancelled else { return }
                visible = true
            }
            .onDisappear { visible = false }
    }
}

func Oxanium(_ size: CGFloat = 16) -> Font { LWTheme.text(size) }

struct ThemeProvider: ViewModifier {
    @AppStorage("lwAppearance") private var appearance = "system"
    @AppStorage("lwSignal") private var signal = "blue"
    @AppStorage("lwReduceMotion") private var reduceMotion = false
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    func body(content: Content) -> some View {
        content.font(LWTheme.text()).foregroundStyle(LWTheme.ink).tint(signal == "mono" ? LWTheme.ink : LWTheme.signal)
            .background(LWTheme.paper)
            .preferredColorScheme(appearance == "system" ? nil : appearance == "dark" ? .dark : .light)
            .environment(\.lwReduceMotion, reduceMotion || systemReduceMotion)
            .buttonStyle(.plain)
            .transaction {
                if reduceMotion || systemReduceMotion {
                    $0.disablesAnimations = true
                    $0.animation = nil
                }
            }
    }
}

struct LWIconButtonStyle: ButtonStyle {
    @Environment(\.lwReduceMotion) private var reduceMotion
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(configuration.isPressed ? LWTheme.panel : Color.clear)
            .contentShape(Rectangle())
            .animation(reduceMotion ? nil : LWMotion.out(LWMotion.quick), value: configuration.isPressed)
    }
}

struct LWKeyStyle: ButtonStyle {
    @Environment(\.lwReduceMotion) private var reduceMotion
    var pressedOpacity = 1.0
    var background: Color? = nil
    var pressedBackground: Color? = nil
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(configuration.isPressed ? pressedBackground ?? background : background)
            .animation(reduceMotion ? nil : LWMotion.out(configuration.isPressed ? LWMotion.instant : LWMotion.quick), value: configuration.isPressed)
            .contentShape(Rectangle())
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.96 : 1)
            .offset(y: configuration.isPressed && !reduceMotion ? 1 : 0)
            .animation(reduceMotion ? nil : LWMotion.springAnimation(configuration.isPressed ? LWMotion.instant : LWMotion.base), value: configuration.isPressed)
            .opacity(configuration.isPressed ? pressedOpacity : 1)
    }
}

struct LWArtworkMatch: ViewModifier {
    @Environment(\.lwArtworkNamespace) private var namespace
    var enabled = true
    @ViewBuilder func body(content: Content) -> some View {
        if enabled, let namespace { content.matchedGeometryEffect(id: "now-artwork", in: namespace).transition(.identity) }
        else { content.transition(.identity) }
    }
}

private struct LWArtworkNamespaceKey: EnvironmentKey { static let defaultValue: Namespace.ID? = nil }
extension EnvironmentValues {
    var lwArtworkNamespace: Namespace.ID? {
        get { self[LWArtworkNamespaceKey.self] }
        set { self[LWArtworkNamespaceKey.self] = newValue }
    }
}

extension View {
    func applyTheme() -> some View { modifier(ThemeProvider()) }
}

private struct LWMatrixLayout: Layout {
    let width: CGFloat
    let height: CGFloat

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        guard width > 0, height > 0 else { return .zero }
        // Like the reference SVG, the display has an intrinsic size during a stack's zero-size probe.
        let fittedWidth = proposal.width.flatMap { $0.isFinite && $0 > 0 ? min(width, $0) : nil } ?? width
        return CGSize(width: fittedWidth, height: height * fittedWidth / width)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        subviews.first?.place(at: bounds.origin, anchor: .topLeading, proposal: ProposedViewSize(bounds.size))
    }
}

struct MatrixText: View {
    let text: String
    var height: CGFloat
    var unlit = false
    @ScaledMetric(relativeTo: .largeTitle) private var displayScale: CGFloat = 1
    @Environment(\.lwReduceMotion) private var reduceMotion
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @State private var previous = ""
    @State private var changedAt = Date.distantPast
    @State private var settledChange = Date.distantPast
    fileprivate static let glyphs: [Character: [String]] = [
        "0":["01110","10001","10001","10001","10001","10001","01110"],
        "1":["00100","01100","00100","00100","00100","00100","01110"],
        "2":["01110","10001","00001","00010","00100","01000","11111"],
        "3":["11111","00010","00100","00010","00001","10001","01110"],
        "4":["00010","00110","01010","10010","11111","00010","00010"],
        "5":["11111","10000","11110","00001","00001","10001","01110"],
        "6":["00110","01000","10000","11110","10001","10001","01110"],
        "7":["11111","00001","00010","00100","01000","01000","01000"],
        "8":["01110","10001","10001","01110","10001","10001","01110"],
        "9":["01110","10001","10001","01111","00001","00010","01100"],
        ":":["0","0","1","0","1","0","0"], ".":["0","0","0","0","0","0","1"],
        "-":["000","000","000","111","000","000","000"],
        "/":["001","001","010","010","010","100","100"], " ":["00","00","00","00","00","00","00"]
    ]
    private var columns: Int { max(1, text.reduce(0) { $0 + (Self.glyphs[$1]?.first?.count ?? 0) + 1 } - 1) }
    private var changeTimes: [Date] {
        if reduceMotion { return [] }
        var times: [Date] = [changedAt]
        for row in 0..<7 {
            let base: TimeInterval = Double(row) * 0.016
            times.append(changedAt.addingTimeInterval(base + LWMotion.quick / 2))
            times.append(changedAt.addingTimeInterval(base + LWMotion.quick))
        }
        return times.sorted()
    }

    var body: some View {
        LWMatrixLayout(width: (CGFloat(columns) - 0.22) * height * displayScale / 7, height: height * displayScale) {
        TimelineView(.explicit(changeTimes)) { timeline in
            Canvas { context, size in
                let unit = min(size.height / 7, size.width / (CGFloat(columns) - 0.22))
                var x: CGFloat = 0
                let old = Array(previous)
                for (index, character) in text.enumerated() {
                    guard let glyph = Self.glyphs[character] else { continue }
                    for (rowIndex, row) in glyph.enumerated() {
                        for (column, cell) in row.enumerated() {
                            let changed = changedAt != .distantPast && (index >= old.count || old[index] != character)
                            let age = timeline.date.timeIntervalSince(changedAt) - Double(rowIndex) * 0.016
                            let fade = reduceMotion || !changed || settledChange == changedAt ? 1.0 : LWMotion.steppedProgress(age, duration: LWMotion.quick, count: 2)
                            let lit = cell == "1"
                            if lit || unlit {
                                let rect = CGRect(x: x + CGFloat(column) * unit, y: CGFloat(rowIndex) * unit, width: unit * 0.78, height: unit * 0.78)
                                context.fill(Path(rect), with: .color(LWTheme.ink.opacity(lit ? fade : 0.035)))
                            }
                        }
                    }
                    x += CGFloat((glyph.first?.count ?? 0) + 1) * unit
                }
            }
        }
        }
        .fixedSize(horizontal: dynamicTypeSize <= .large, vertical: dynamicTypeSize <= .large)
        .accessibilityLabel(text)
        .onChange(of: text) { old, _ in previous = old; changedAt = Date() }
        .task(id: changedAt) {
            guard changedAt != .distantPast, !reduceMotion else { return }
            let change = changedAt
            let end = change.addingTimeInterval(LWMotion.quick + 6 * 0.016)
            do { try await Task.sleep(for: .seconds(max(0, end.timeIntervalSinceNow))) }
            catch { return }
            guard !Task.isCancelled, changedAt == change else { return }
            // Explicit timelines can deliver the final boundary with the preceding display date.
            // Invalidate the Canvas once at completion so its last row cannot remain half lit.
            settledChange = change
        }
    }
}

struct LWArtwork: View {
    var song: Song?
    var size: CGFloat
    var matched = false
    nonisolated static func thumbnail(at url: URL, maxPixelSize: Int) -> UIImage? {
        guard maxPixelSize > 0,
              let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceThumbnailMaxPixelSize: maxPixelSize,
                kCGImageSourceShouldCacheImmediately: true
              ] as CFDictionary) else { return nil }
        return UIImage(cgImage: image)
    }
    @State private var image: UIImage?
    @Environment(\.displayScale) private var displayScale
    private var imagePixelSize: Int { max(1, Int(ceil(size * displayScale))) }
    private var identity: String { song?.coverArtPath ?? song?.album ?? "" }
    var body: some View {
        ZStack {
            if let image { Image(uiImage: image).resizable().interpolation(.high).scaledToFill() }
            else {
                Canvas { context, bounds in
                    let seed = identity.utf8.reduce(UInt64(2166136261)) { ($0 ^ UInt64($1)) &* 16777619 }
                    let unit = bounds.width / 100
                    let style = Int(seed % 10)
                    let dark = [1, 5, 6].contains(style)
                    let accent = Color(red: 35 / 255, green: 64 / 255, blue: 1)
                    let background = style == 8 ? accent : Color(white: dark ? 0.07 : style == 3 ? 0.83 : 0.91)
                    let foreground = Color(white: dark || style == 8 ? 0.95 : 0.07)
                    context.fill(Path(CGRect(origin: .zero, size: bounds)), with: .color(background))
                    switch style {
                    case 0:
                        context.fill(Path(ellipseIn: CGRect(x: unit * 14, y: unit * 12, width: unit * 72, height: unit * 72)), with: .color(foreground.opacity(0.14)))
                        context.fill(Path(ellipseIn: CGRect(x: unit * 43, y: unit * 41, width: unit * 14, height: unit * 14)), with: .color(background))
                        var line = Path(); line.move(to: CGPoint(x: 22 * unit, y: 76 * unit)); line.addLine(to: CGPoint(x: 78 * unit, y: 20 * unit))
                        context.stroke(line, with: .color(foreground.opacity(0.4)), lineWidth: max(0.4, unit * 0.5))
                    case 1:
                        for index in 0..<26 {
                            let random = Double((seed &+ UInt64(index) &* 2654435761) % 1000) / 1000
                            let h = (8 + random * 44 * (1 - abs(Double(index) - 13) / 20)) * unit
                            let rect = CGRect(x: (10 + CGFloat(index) * 3.1) * unit, y: 78 * unit - h, width: unit * 1.2, height: h)
                            context.fill(Path(rect), with: .color(index == 17 ? accent : foreground))
                        }
                    case 2:
                        for row in 0..<6 {
                            for column in 0..<6 {
                                let rect = CGRect(x: (12 + CGFloat(column) * 13) * unit, y: (10 + CGFloat(row) * 13) * unit, width: 11 * unit, height: 11 * unit)
                                if (seed &+ UInt64(row * 6 + column) &* 2654435761) % 10 < 4 { context.fill(Path(rect), with: .color(foreground)) }
                                context.stroke(Path(rect), with: .color(foreground.opacity(0.5)), lineWidth: max(0.4, unit * 0.3))
                            }
                        }
                    case 3:
                        for index in 0..<30 {
                            let length = CGFloat((seed &+ UInt64(index) &* 2654435761) % 60) + 20
                            context.fill(Path(CGRect(x: unit * 10, y: (12 + CGFloat(index) * 2.6) * unit, width: length * unit, height: max(0.5, unit * 0.6))), with: .color(foreground))
                        }
                    case 4:
                        context.fill(Path(CGRect(x: 34 * unit, y: 36 * unit, width: 58 * unit, height: 42 * unit)), with: .color(accent))
                        context.fill(Path(CGRect(x: 8 * unit, y: 29 * unit, width: 28 * unit, height: unit)), with: .color(foreground))
                        context.fill(Path(CGRect(x: 38 * unit, y: 48 * unit, width: 5 * unit, height: 1.6 * unit)), with: .color(foreground))
                        context.fill(Path(CGRect(x: 38 * unit, y: 64 * unit, width: 10 * unit, height: 1.6 * unit)), with: .color(foreground))
                    case 5:
                        for index in 1...7 {
                            let radius = CGFloat(index) * 5.6 * unit
                            let rect = CGRect(x: bounds.width / 2 - radius, y: bounds.height / 2 - radius, width: radius * 2, height: radius * 2)
                            context.stroke(Path(ellipseIn: rect), with: .color(index == 5 ? accent : foreground.opacity(0.55)), lineWidth: index == 5 ? 1.2 * unit : max(0.4, 0.4 * unit))
                        }
                    case 6:
                        context.fill(Path(ellipseIn: CGRect(x: 14 * unit, y: 12 * unit, width: 72 * unit, height: 72 * unit)), with: .color(Color(white: 0.16)))
                        var arc = Path()
                        arc.addArc(center: CGPoint(x: 50 * unit, y: 48 * unit), radius: 36 * unit, startAngle: .degrees(-90), endAngle: .degrees(0), clockwise: false)
                        context.stroke(arc, with: .color(accent), lineWidth: 1.6 * unit)
                        context.fill(Path(ellipseIn: CGRect(x: 45 * unit, y: 43 * unit, width: 10 * unit, height: 10 * unit)), with: .color(background))
                    case 7, 8:
                        let label = String(format: "%02d", Int(seed % 100))
                        let pixel = (style == 7 ? 48.0 : 38.0) * unit / 7
                        let labelWidth = 11 * pixel
                        var x = (bounds.width - labelWidth) / 2
                        for character in label {
                            if let glyph = MatrixText.glyphs[character] {
                                for (row, cells) in glyph.enumerated() {
                                    for (column, cell) in cells.enumerated() where cell == "1" {
                                        context.fill(Path(CGRect(x: x + CGFloat(column) * pixel, y: 25 * unit + CGFloat(row) * pixel, width: pixel * (style == 7 ? 1 : 0.78), height: pixel * (style == 7 ? 1 : 0.78))), with: .color(foreground))
                                    }
                                }
                            }
                            x += 6 * pixel
                        }
                    default:
                        for row in 0..<7 {
                            for column in 0..<6 where (row + column).isMultiple(of: 2) {
                                context.fill(Path(CGRect(x: (10 + CGFloat(column) * 2.2) * unit, y: (30 + CGFloat(row) * 7 + CGFloat(column % 2) * 1.1) * unit, width: 2.2 * unit, height: 2.2 * unit)), with: .color(foreground))
                            }
                        }
                        var slash = Path(); slash.move(to: CGPoint(x: 60 * unit, y: 54 * unit)); slash.addLine(to: CGPoint(x: 74 * unit, y: 68 * unit))
                        context.stroke(slash, with: .color(foreground), lineWidth: unit)
                    }
                    if style == 2 {
                        context.fill(Path(CGRect(x: 64 * unit, y: 75 * unit, width: 11 * unit, height: 11 * unit)), with: .color(accent))
                    }
                    if [0, 3, 7, 9].contains(style) {
                        context.fill(Path(CGRect(x: 80 * unit, y: 10 * unit, width: unit * 5, height: unit * 5)), with: .color(accent))
                    }
                    let artist = (song?.albumArtist.isEmpty == false ? song?.albumArtist : song?.artist) ?? ""
                    context.draw(Text(String(artist.lowercased().prefix(32))).font(.custom("GeistMono-Regular", fixedSize: 3.6 * unit)).foregroundColor(foreground.opacity(0.5)), at: CGPoint(x: 6 * unit, y: 95 * unit), anchor: .bottomLeading)
                }
            }
        }
        .clipped()
        .modifier(LWArtworkMatch(enabled: matched))
        .frame(width: size, height: size).accessibilityHidden(true)
        .task(id: identity + ":" + String(imagePixelSize)) {
            guard let path = song?.coverArtPath else { image = nil; return }
            let url = path.hasPrefix("/") ? URL(fileURLWithPath: path) : FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent(path)
            let pixels = imagePixelSize
            let thumbnail = await Task.detached(priority: .utility) { Self.thumbnail(at: url, maxPixelSize: pixels) }.value
            guard !Task.isCancelled else { return }
            image = thumbnail
        }
    }
}

private struct LWReduceMotionKey: EnvironmentKey { static let defaultValue = false }
extension EnvironmentValues {
    var lwReduceMotion: Bool {
        get { self[LWReduceMotionKey.self] }
        set { self[LWReduceMotionKey.self] = newValue }
    }
}

private struct LWWideLayoutKey: EnvironmentKey { static let defaultValue = false }
extension EnvironmentValues {
    var lwWideLayout: Bool {
        get { self[LWWideLayoutKey.self] }
        set { self[LWWideLayoutKey.self] = newValue }
    }
}

private struct LWFolderFilterKey: EnvironmentKey { static let defaultValue: Binding<String>? = nil }
extension EnvironmentValues {
    var lwFolderFilter: Binding<String>? {
        get { self[LWFolderFilterKey.self] }
        set { self[LWFolderFilterKey.self] = newValue }
    }
}

#endif
