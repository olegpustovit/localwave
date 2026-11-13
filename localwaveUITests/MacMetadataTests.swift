#if os(macOS)
import XCTest

final class MacMetadataTests: XCTestCase {
    func testArtistsAndFolderShowTrackMetadata() {
        // UI qualification is manual/on demand: two native windows and their screenshot attachments.
        for (screen, title) in [("artists", "Halden Marsh"), ("folder", "Blue Hour")] {
            let app = XCUIApplication()
            app.launchEnvironment["LOCALWAVE_UI_TEST_ID"] = UUID().uuidString
            app.launchEnvironment["LOCALWAVE_REFERENCE_FIXTURE"] = "1"
            app.launchEnvironment["LOCALWAVE_REFERENCE_SCREEN"] = screen
            app.launchArguments = ["-lwReduceMotion", "YES"]
            app.launch()
            let row = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", title)).firstMatch
            XCTAssertTrue(row.waitForExistence(timeout: 15))
            if screen == "folder" {
                XCTAssertTrue(row.label.contains("Halden Marsh · Field Recordings 04"))
                XCTAssertFalse(row.label.contains("01 Blue Hour.flac"))
                XCTAssertTrue(app.buttons["Play folder"].firstMatch.exists)
            }
            let image = XCTAttachment(screenshot: app.screenshot())
            image.name = "Mac metadata — " + screen
            image.lifetime = .keepAlways
            add(image)
            app.terminate()
        }
    }
}
#endif
