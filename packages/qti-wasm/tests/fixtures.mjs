export const bbqText = "MC\t<p>Identify the image: <img src=\"pixel.png\" alt=\"pixel\" /> alpha</p>\talpha\tCorrect\tbeta\tIncorrect\n";
export const bbqBytes = new TextEncoder().encode(bbqText);
export const pixelBytes = new Uint8Array(Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==", "base64"));

export function request(outputFormat, outputName) {
  return {
    inputFormat: "bbq_text_upload",
    outputFormat,
    input: { kind: "file", name: "questions.txt", bytes: bbqBytes, companions: [{ name: "pixel.png", bytes: pixelBytes }] },
    allowMixed: true,
    outputName,
    document: { title: "Fixed fixture", date: "2026-01-01" },
    shuffleSeed: 0,
  };
}

export function artifactFiles(artifact) {
  if (artifact === null) return [];
  return artifact.kind === "file" ? [artifact.primary, ...artifact.companions] : artifact.entries;
}
