import Testing
@testable import localwave

struct SongDragPayloadTests {
    @Test func orderedGroupsRoundTripAndLegacySinglePayloadStillWorks() {
        #expect(LWSongDragPayload.encode([8]) == "localwave-song:8")
        #expect(LWSongDragPayload.encode([9, 3, 9, 4]) == "localwave-songs:9,3,4")
        #expect(LWSongDragPayload.decode([LWSongDragPayload.encode([9, 3, 9, 4])]) == [9, 3, 4])
        #expect(LWSongDragPayload.decode(["localwave-song:7", "localwave-songs:3,7,2", "localwave-song:9"]) == [7, 3, 2, 9])
    }

    @Test func malformedGroupsAreRejectedWithoutCorruptingOtherItemsOrAcceptingQueueEntries() {
        #expect(LWSongDragPayload.decode(["localwave-songs:1,nope,2", "localwave-song:4"]) == [4])
        #expect(LWSongDragPayload.decode(["localwave-songs:1,,2", "localwave-song:2,3", "localwave-songs:0,3", "localwave-song:-8", "localwave-songs:"]).isEmpty)
        #expect(LWSongDragPayload.decode(["localwave-queue:9B4AB4D5-9B0F-4EB5-9956-C447E972D981", "1,2", "Track title"]).isEmpty)
    }
}
