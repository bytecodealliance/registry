export namespace ExportsComponentHashCheckpointHashing {
  export function hashCheckpoint(contents: Contents, keyId: KeyId, signature: Signature): string;
}
export interface Contents {
  logRoot: string,
  logLength: number,
  mapRoot: string,
}
export type KeyId = string;
export type Signature = string;
