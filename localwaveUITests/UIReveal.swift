import XCTest

extension XCTestCase {
    @discardableResult
    func reveal(_ element: XCUIElement, in app: XCUIApplication, fullyInsideScroll: Bool = false,
                maxSwipes: Int = 6, file: StaticString = #filePath, line: UInt = #line) -> Bool {
        for attempt in 0...maxSwipes {
            let exists = element.exists
            if exists && element.isHittable && !fullyInsideScroll { return true }
            guard let scroll = app.scrollViews.allElementsBoundByIndex.first(where: {
                $0.exists && $0.isHittable && $0.frame.height > 100 && $0.frame.width > 100
            }) else {
                XCTFail("No visible outer scroll view can reveal the target.", file: file, line: line)
                return false
            }
            let viewport = scroll.frame.intersection(app.frame).insetBy(dx: 4, dy: 4)
            let targetFrame = exists ? element.frame : .zero
            if exists && element.isHittable && viewport.contains(targetFrame) { return true }
            guard attempt < maxSwipes else { break }
            let scrollDown = exists && !targetFrame.isEmpty && targetFrame.minY < viewport.minY
            // The edge stays outside padded waveform/slider drag regions.
            let x = viewport.minX + 4
            let top = viewport.minY + viewport.height * 0.2
            let bottom = viewport.minY + viewport.height * 0.8
            let origin = app.coordinate(withNormalizedOffset: .zero)
            let start = origin.withOffset(CGVector(dx: x, dy: scrollDown ? top : bottom))
            let end = origin.withOffset(CGVector(dx: x, dy: scrollDown ? bottom : top))
            start.press(forDuration: 0.01, thenDragTo: end)
        }
        XCTFail("Target did not become visible after \(maxSwipes) scroll gestures.", file: file, line: line)
        return false
    }
}
