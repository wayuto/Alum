import { isalpha } from "./utils.ts";

/**Preprocessor */
export class Preprocessor {
  private pos: number = 0;

  constructor(private src: string) {}

  private current = (): string =>
    this.pos < this.src.length ? this.src[this.pos] : "\0";

  private bump = (): void => {
    this.pos++;
  };

  private skipSpaces = (): void => {
    while (
      this.current() == " " || this.current() == "\t" || this.current() == "\n"
    ) this.bump();
  };

  private parseAlpha = (): string => {
    let alpha: string = "";
    while (isalpha(this.current())) {
      alpha += this.current();
      this.bump();
    }
    return alpha;
  };

  private parseFilePath = (): string | undefined => {
    this.skipSpaces();
    if (this.current() !== '"') {
      return undefined;
    }

    this.bump();

    let file = "";
    while (this.current() !== '"' && this.current() !== "\0") {
      file += this.current();
      this.bump();
    }

    if (this.current() === '"') {
      this.bump();
      return file;
    }

    return undefined;
  };

  public preprocess = async (): Promise<string> => {
    let output: string = "";

    while (this.current() !== "\0") {
      output += (() => {
        let chunk: string = "";
        while (this.current() !== "\0" && this.current() !== "$") {
          chunk += this.current();
          this.bump();
        }
        return chunk;
      })();

      if (this.current() === "\0") {
        break;
      }

      if (this.current() === "$") {
        const startPos = this.pos;
        this.bump();
        const command = this.parseAlpha();

        switch (command) {
          case "import": {
            const file = this.parseFilePath();

            if (!file) {
              this.pos = startPos;
              output += this.current();
              this.bump();
              break;
            }

            const raw = await Deno.readTextFile(file);
            const pp = new Preprocessor(raw);
            const content = await pp.preprocess();

            output += content;
            break;
          }
          default: {
            this.pos = startPos;
            output += this.current();
            this.bump();
            break;
          }
        }
      }
    }
    return output;
  };
}
