export interface Content {
  logRoot: string,
  logLength: number,
  mapRoot: string,
}
export interface Checkpoint {
  contents: Content,
  keyId: string,
  signature: string,
}
export namespace Storage {
  export function storeRegistryInfo(input: string): void;
  export function getRegistryInfo(): string;
  export function getCheckpoint(): Checkpoint;
  export function hashCheckpoint(input: Checkpoint): string;
}
