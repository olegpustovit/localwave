import Foundation
import Testing
import LocalWaveDomain
@testable import localwave

struct RootNavigationHistoryTests {
    @Test func interleavedSurfacesRestoreTheirOriginalChildBoundaryAndSearch() {
        let album = PlaybackContext.album(name: "First", artist: "Artist")
        let list = PlaybackContext.playlist(id: 8, name: "Evening")
        var history = LWRootNavigationHistory(initial: LWRootNavigationFrame(section: "Find", query: "cloud", showAll: true))
        history.update(history.current.id) { $0.libraryPath = [album] }
        history.push(LWRootNavigationFrame(section: "Lists"))
        history.update(history.current.id) { $0.libraryPath = [list, album] }
        history.push(LWRootNavigationFrame(section: "Settings"))
        history.push(LWRootNavigationFrame(section: "Source:3", context: .folder(sourceID: 3, pathID: 9, name: "Folder")))
        history.push(LWRootNavigationFrame(section: "Find", query: "year:2019"))
        #expect(history.back()?.section == "Source:3")
        #expect(history.back()?.showsSettings == true)
        #expect(history.back()?.libraryPath == [list, album])
        #expect(history.current.libraryBoundaryDepth == 0)
        let count = history.previous.count
        history.update(history.current.id) { $0.libraryPath = [list] }
        #expect(history.previous.count == count)
        #expect(history.back()?.libraryPath == [album])
        #expect(history.current.libraryBoundaryDepth == 0)
        #expect(history.current.searchQuery == "cloud")
        #expect(history.current.searchShowAll)
        #expect(history.back() == nil)
    }

    @Test func repeatedDestinationsAreDistinctAndOldCallbacksCannotOverwriteCurrent() {
        let album = PlaybackContext.album(name: "Same", artist: "Artist")
        var history = LWRootNavigationHistory()
        let originalID = history.current.id
        history.push(LWRootNavigationFrame(section: "Albums", context: album))
        let firstID = history.current.id
        history.push(LWRootNavigationFrame(section: "Albums", context: album))
        #expect(history.current.id != firstID)
        history.update(originalID) { $0.section = "Stale" }
        history.update(firstID) { $0.libraryPath = [] }
        #expect(history.current.section == "Albums")
        #expect(history.current.libraryBoundaryDepth == 1)
        #expect(history.back()?.id == firstID)
        #expect(history.current.libraryBoundaryDepth == 1)
    }

    @Test func mobileRootResetDiscardsDesktopHistory() {
        var history = LWRootNavigationHistory()
        history.push(LWRootNavigationFrame(section: "Find", query: "cloud"))
        var files = LWRootNavigationFrame(section: "Folders", context: .folder(sourceID: 3, pathID: 9, name: "Album folder"))
        files.includesFolderAncestors = true
        history.reset(files)
        #expect(!history.canGoBack)
        #expect(history.current.includesFolderAncestors)
        #expect(history.current.section == "Folders")
        #expect(history.back() == nil)
    }

    @Test func removedSourcesAndListsDoNotReappearFromHistory() {
        var history = LWRootNavigationHistory()
        history.push(LWRootNavigationFrame(section: "Source:3", context: .folder(sourceID: 3, pathID: 9, name: "Folder")))
        history.push(LWRootNavigationFrame(section: "List:8", context: .playlist(id: 8, name: "Removed")))
        history.reconcile(sourceIDs: [], listIDs: [])
        #expect(history.current.section == "Lists")
        #expect(history.current.libraryBoundaryDepth == 0)
        #expect(history.back()?.section == "Albums")
        #expect(!history.canGoBack)
    }
}
