export const isdigit = (ch: string): boolean =>
  ch.length === 1 && ch >= "0" && ch <= "9";

export const isalpha = (ch: string): boolean =>
  ch.length === 1 && ((ch >= "A" && ch <= "Z") || (ch >= "a" && ch <= "z"));

export const err = (from: string, msg: string): never => {
  throw `${from}: ${msg}`;
};
