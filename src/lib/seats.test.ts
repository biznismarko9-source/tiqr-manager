import { test } from "node:test";
import assert from "node:assert/strict";
import { parseSeats, seatRun, seatsForCount } from "./seats.ts";

test("seats are read as a range or a list", () => {
  assert.deepEqual(parseSeats("12-15"), ["12", "13", "14", "15"]);
  assert.deepEqual(parseSeats("15-13"), ["15", "14", "13"]);
  assert.deepEqual(parseSeats("12, 14, 16A"), ["12", "14", "16A"]);
  assert.deepEqual(parseSeats("  "), []);
});

test("a run of numbered seats starts at its lowest seat", () => {
  assert.equal(seatRun(parseSeats("23")), 23);
  assert.equal(seatRun(parseSeats("26-23")), 23);
  assert.equal(seatRun(parseSeats("23, 25")), null, "a gap");
  assert.equal(seatRun(parseSeats("A1, A2")), null);
  assert.equal(seatRun([]), null);
});

test("a typed count moves numbered seats with it - and leaves anything else alone", () => {
  // marko: "pri new order nejde menit kvantita manualne".
  assert.equal(seatsForCount("23", 3), "23-25");
  assert.equal(seatsForCount("23-26", 2), "23-24");
  assert.equal(seatsForCount("5, 6", 1), "5");
  assert.equal(seatsForCount("", 4), null, "no seats - the count alone");
  assert.equal(seatsForCount("A, B", 3), null, "not numbered - pointed out instead");
  assert.equal(seatsForCount("23", 0), null);
  assert.equal(seatsForCount("23", Number.NaN), null);
  assert.equal(seatsForCount("1", 5000), null, "not a whole stadium by a slip of the keyboard");
});
