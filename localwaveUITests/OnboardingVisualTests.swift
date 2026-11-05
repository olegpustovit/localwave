import XCTest

final class OnboardingVisualTests: XCTestCase {
    func testReferenceOnboardingDisclosureAndLibraryContinuation() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
        app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = "onboarding"
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["Choose a folder"].waitForExistence(timeout: 20))
        XCTAssertFalse(app.buttons["library-mode-Albums"].exists, "Onboarding must hide the underlying library from accessibility")
        let initial = XCTAttachment(screenshot: app.screenshot())
        initial.name = "Reference onboarding normal light"
        initial.lifetime = .keepAlways
        add(initial)
        let drive = app.buttons["Google Drive not in the list?"]
        guard reveal(drive, in: app) else { return }
        drive.tap()
        XCTAssertTrue(app.staticTexts["Install the Google Drive app, then in Files tap Browse → ⋯ → Edit and turn on Google Drive."].waitForExistence(timeout: 5))
        let expanded = XCTAttachment(screenshot: app.screenshot())
        expanded.name = "Reference onboarding Drive help expanded"
        expanded.lifetime = .keepAlways
        add(expanded)
        let explore = app.buttons["Try the demo library"]
        guard reveal(explore, in: app) else { return }
        explore.tap()
        XCTAssertTrue(app.buttons["library-mode-Albums"].waitForExistence(timeout: 10))
    }

    func testRealDemoImportsAndPlaysWithoutDuplicatingAnExistingLibrary() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launchEnvironment["LOCALWAVE_UI_EMPTY_LIBRARY"] = "1"
        app.launch()
        defer { app.terminate() }
        let demo = app.buttons["Try the demo library"]
        XCTAssertTrue(demo.waitForExistence(timeout: 25))
        demo.tap()
        let albums = app.buttons["library-mode-Albums"]
        XCTAssertTrue(albums.waitForExistence(timeout: 60))
        let count = app.descendants(matching: .any).matching(identifier: "library-song-count").firstMatch
        let populated = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", "92"), object: count)
        XCTAssertEqual(XCTWaiter.wait(for: [populated], timeout: 30), .completed)
        app.buttons["Folders"].firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Demo Library"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertEqual(app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "source.open.")).count, 1)
        app.buttons["Library"].firstMatch.tap()
        let album = app.staticTexts["Field Recordings 04"].firstMatch
        guard reveal(album, in: app) else { return }
        album.tap()
        let play = app.buttons["library-play-context"]
        XCTAssertTrue(play.waitForExistence(timeout: 10))
        play.tap()
        XCTAssertTrue(app.buttons["Pause"].firstMatch.waitForExistence(timeout: 15))
        app.buttons["Open now playing"].firstMatch.tap()
        let advancing = app.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH %@ AND label != %@", "Elapsed time ", "Elapsed time 0:00")).firstMatch
        XCTAssertTrue(advancing.waitForExistence(timeout: 10), "The bundled recording must actually advance the native playback clock")
        app.buttons["Pause"].firstMatch.tap()
        app.terminate()

        app.launch()
        let explore = app.buttons["Explore the library"]
        XCTAssertTrue(explore.waitForExistence(timeout: 25))
        let ready = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"), object: explore)
        XCTAssertEqual(XCTWaiter.wait(for: [ready], timeout: 5), .completed)
        explore.tap()
        XCTAssertTrue(albums.waitForExistence(timeout: 10))
        XCTAssertEqual(count.value as? String, "92")
        app.buttons["Folders"].firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Demo Library"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertEqual(app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "source.open.")).count, 1)
    }
}
