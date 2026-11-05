import Foundation
import XCTest
@testable import LocalWaveCore

final class PreferencesTests: XCTestCase {
    func testUITestPreferencesDoNotChangeNormalStateOrAnotherRun() {
        let firstID = UUID(), secondID = UUID()
        let first = LocalWavePreferences.store(testID: firstID.uuidString)
        let second = LocalWavePreferences.store(testID: secondID.uuidString)
        defer {
            first.removePersistentDomain(forName: "com.olegpustovit.localwave.ui-test." + firstID.uuidString)
            second.removePersistentDomain(forName: "com.olegpustovit.localwave.ui-test." + secondID.uuidString)
        }
        let key = "fixture-" + UUID().uuidString
        first.set(101, forKey: key)
        second.set(202, forKey: key)
        XCTAssertEqual(first.integer(forKey: key), 101)
        XCTAssertEqual(second.integer(forKey: key), 202)
        XCTAssertNil(UserDefaults.standard.object(forKey: key))
        XCTAssertTrue(LocalWavePreferences.store(testID: nil) === UserDefaults.standard)
        XCTAssertTrue(LocalWavePreferences.store(testID: "not-a-test-uuid") === UserDefaults.standard)
    }
}
