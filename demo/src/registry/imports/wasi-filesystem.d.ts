export type Descriptor = number;
export type Filesize = bigint;
export type OutputStream = OutputStream;
/**
 * # Variants
 * 
 * ## `"access"`
 * 
 * ## `"again"`
 * 
 * ## `"already"`
 * 
 * ## `"badf"`
 * 
 * ## `"busy"`
 * 
 * ## `"deadlk"`
 * 
 * ## `"dquot"`
 * 
 * ## `"exist"`
 * 
 * ## `"fbig"`
 * 
 * ## `"ilseq"`
 * 
 * ## `"inprogress"`
 * 
 * ## `"intr"`
 * 
 * ## `"inval"`
 * 
 * ## `"io"`
 * 
 * ## `"isdir"`
 * 
 * ## `"loop"`
 * 
 * ## `"mlink"`
 * 
 * ## `"msgsize"`
 * 
 * ## `"nametoolong"`
 * 
 * ## `"nodev"`
 * 
 * ## `"noent"`
 * 
 * ## `"nolck"`
 * 
 * ## `"nomem"`
 * 
 * ## `"nospc"`
 * 
 * ## `"nosys"`
 * 
 * ## `"notdir"`
 * 
 * ## `"notempty"`
 * 
 * ## `"notrecoverable"`
 * 
 * ## `"notsup"`
 * 
 * ## `"notty"`
 * 
 * ## `"nxio"`
 * 
 * ## `"overflow"`
 * 
 * ## `"perm"`
 * 
 * ## `"pipe"`
 * 
 * ## `"rofs"`
 * 
 * ## `"spipe"`
 * 
 * ## `"txtbsy"`
 * 
 * ## `"xdev"`
 */
export type Errno = 'access' | 'again' | 'already' | 'badf' | 'busy' | 'deadlk' | 'dquot' | 'exist' | 'fbig' | 'ilseq' | 'inprogress' | 'intr' | 'inval' | 'io' | 'isdir' | 'loop' | 'mlink' | 'msgsize' | 'nametoolong' | 'nodev' | 'noent' | 'nolck' | 'nomem' | 'nospc' | 'nosys' | 'notdir' | 'notempty' | 'notrecoverable' | 'notsup' | 'notty' | 'nxio' | 'overflow' | 'perm' | 'pipe' | 'rofs' | 'spipe' | 'txtbsy' | 'xdev';
export namespace WasiFilesystem {
  export function writeViaStream(this: Descriptor, offset: Filesize): OutputStream;
  export function appendViaStream(this: Descriptor): OutputStream;
  export function dropDescriptor(this: Descriptor): void;
}
