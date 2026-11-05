import Foundation
import CoreTransferable
import UniformTypeIdentifiers
import Testing
@testable import localwave

struct QueueDragPayloadTests {
    @Test func queueOccurrenceTransferKeepsUUIDAndCannotClaimTextSongDrops() throws {
        func acceptsTransferable<T: Transferable & Codable>(_: T.Type) {}
        acceptsTransferable(LWQueueEntryDragPayload.self)
        let first = LWQueueEntryDragPayload(entryID: UUID())
        let duplicateSongOccurrence = LWQueueEntryDragPayload(entryID: UUID())
        let decoded = try #require(LWQueueEntryDragPayload.decode(first.encodedData))
        #expect(String(data: first.encodedData, encoding: .utf8) == first.entryID.uuidString)
        #expect(decoded.entryID == first.entryID)
        #expect(decoded.entryID != duplicateSongOccurrence.entryID)
        #expect(LWQueueEntryDragPayload.decode(Data("not-a-UUID".utf8)) == nil)
        #expect(LWQueueEntryDragPayload.contentType.conforms(to: .data))
        #expect(!LWQueueEntryDragPayload.contentType.conforms(to: .text))
        #expect(!LWQueueEntryDragPayload.contentType.conforms(to: .plainText))
    }
    @Test func queueDropUsesOnlyUpcomingFramesAndClampsToNearestRowCenter() {
        let current = UUID(), first = UUID(), last = UUID(), stale = UUID()
        let frames = [current: CGRect(x: 0, y: 0, width: 200, height: 44),
                      first: CGRect(x: 0, y: 100, width: 200, height: 44),
                      last: CGRect(x: 0, y: 144, width: 200, height: 44),
                      stale: CGRect(x: 0, y: 115, width: 200, height: 44)]
        let upcoming = [first, last]
        #expect(LWQueueDropPlacement.targetID(at: CGPoint(x: 20, y: 0), frames: frames, upcomingIDs: upcoming) == first)
        #expect(LWQueueDropPlacement.targetID(at: CGPoint(x: 20, y: 1000), frames: frames, upcomingIDs: upcoming) == last)
        #expect(LWQueueDropPlacement.targetID(at: CGPoint(x: 20, y: 133), frames: frames, upcomingIDs: upcoming) == first)
        #expect(LWQueueDropPlacement.targetID(at: CGPoint(x: 20, y: 160), frames: frames, upcomingIDs: upcoming) == last)
        #expect(LWQueueDropPlacement.targetID(at: .zero, frames: frames, upcomingIDs: []) == nil)
    }
}
