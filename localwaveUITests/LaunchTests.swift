import XCTest

final class LaunchTests: XCTestCase {
    func testLibraryAndSyncNavigation() {
        let app = XCUIApplication()
        app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
        app.launch()
        XCTAssertTrue(app.buttons["Songs"].waitForExistence(timeout: 15))
        app.buttons["Songs"].tap()
        let song = app.staticTexts["Playlist Test Track"]
        XCTAssertTrue(song.waitForExistence(timeout: 10))
        song.press(forDuration: 1.2)
        app.buttons["Add to Playlist"].tap()
        XCTAssertTrue(app.navigationBars["Select Playlist"].waitForExistence(timeout: 5))
        app.buttons["UI Test Playlist"].tap()
        XCTAssertTrue(app.buttons["Library"].waitForExistence(timeout: 5))
        app.navigationBars.buttons["Library"].tap()
        app.buttons["Playlists"].tap()
        app.buttons["UI Test Playlist"].tap()
        XCTAssertTrue(song.waitForExistence(timeout: 5))
        app.buttons["Sync"].tap()
        XCTAssertTrue(app.navigationBars["Music Sources"].waitForExistence(timeout: 10))
    }
}
