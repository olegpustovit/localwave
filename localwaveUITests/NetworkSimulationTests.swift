import XCTest

final class NetworkSimulationTests: XCTestCase {
    func testReferenceToggleShowsOfflineBannerAndGoOnlineClearsIt() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "settings"
        app.launch()
        defer { app.terminate() }
        let toggle = app.switches["Simulate no network"]
        XCTAssertTrue(toggle.waitForExistence(timeout: 25))
        XCTAssertEqual(toggle.value as? String, "0")
        toggle.tap()
        XCTAssertEqual(toggle.value as? String, "1")
        let banner = app.buttons["You’re offline · cloud songs are skipped. Go online"]
        XCTAssertTrue(banner.waitForExistence(timeout: 5))
        XCTAssertEqual(banner.frame.height, 36, accuracy: 0.34)
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Reference Settings with simulated offline banner"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        banner.tap()
        let gone = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: banner)
        XCTAssertEqual(XCTWaiter.wait(for: [gone], timeout: 5), .completed)
        XCTAssertEqual(toggle.value as? String, "0")
    }

    func testActualLocalAudioPlaysWhileNetworkIsSimulatedUnavailable() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 20))
        app.buttons["Settings"].tap()
        let toggle = app.switches["Simulate no network"]
        XCTAssertTrue(toggle.waitForExistence(timeout: 5))
        toggle.tap()
        app.buttons["Library"].firstMatch.tap()
        let songs = app.buttons["library-mode-Songs"]
        XCTAssertTrue(songs.waitForExistence(timeout: 10))
        guard reveal(songs, in: app, fullyInsideScroll: true) else { return }
        songs.tap()
        let track = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "play-song-")).firstMatch
        XCTAssertTrue(track.waitForExistence(timeout: 15))
        guard reveal(track, in: app, fullyInsideScroll: true) else { return }
        track.tap()
        XCTAssertTrue(app.buttons["Pause"].firstMatch.waitForExistence(timeout: 15))
        XCTAssertTrue(app.buttons["You’re offline · cloud songs are skipped. Go online"].exists)
        XCTAssertFalse(app.alerts.firstMatch.exists)
    }
}
