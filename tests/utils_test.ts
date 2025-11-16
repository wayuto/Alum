import { assertEquals } from "@std/assert";
import { isalpha, isdigit } from "../src/utils.ts";

Deno.test("Utlis", () => {
  assertEquals(isalpha("a"), true);
  assertEquals(isalpha("1"), false);
  assertEquals(isdigit("1"), true);
  assertEquals(isdigit("a"), false);
});
