import XCTest
@testable import localwave

@MainActor
final class SourceScanStateTests: XCTestCase {
    func testScanAndImportHaveDistinctStatuses() {
        let state = SourceScanState()
        XCTAssertEqual(state.statusLabel, "scanning…")
        state.progress = 0
        XCTAssertEqual(state.statusLabel, "importing…")
        state.progress = 100
        XCTAssertEqual(state.statusLabel, "importing…")
        state.progress = nil
        XCTAssertEqual(state.statusLabel, "scanning…")
    }
}
