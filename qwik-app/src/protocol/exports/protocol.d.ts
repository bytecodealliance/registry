export interface ProtoEnvelopeBody {
  contentBytes: Uint8Array,
  keyId: string,
  signature: string,
}
export namespace Protocol {
  export function validate(packageRecord: ProtoEnvelopeBody): void;
}
