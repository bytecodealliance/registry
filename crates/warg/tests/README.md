# Log Tests

The `package.rs` and `operator.rs` modules

1. read in tests from the `package-logs` and `operator-logs` directories respectively,
2. construct signed logs using the contents,
3. attempt to validate them, and
4. compare the validation output to what was specified in the test file.

## Keys

Log tests need private keys to sign envelopes,
public keys to include in e.g. `Init` and `GrantFlat` entries,
and fingerprints to use in e.g. `RevokeFlat` entries.

These Keys have been generated randomly using ECDSA and the P-256 curve for test purposes only.

* Alice
  * Private Key: `ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk=`
  * Public Key: `ecdsa-p256:A1OfZz5Y9Ny7VKPVwroCTQPAr9tmlI4U/UTYHZHA87AF`
  * Fingerprint: `sha256:d6d9b4cd077a829c0275233bf3843c8294e250dfcc82b8ea15745e92982a820d`
* Bob
  * Private Key: `ecdsa-p256:2CV1EpLaSYEn4In4OAEDAj5O4Hzu8AFAxgHXuG310Ew=`
  * Public Key: `ecdsa-p256:A5qc6uBi070EBb4GihGzpx6Cm5+oZnv4dWpBhhuZVagu`
  * Fingerprint: `sha256:8ed824821ce75c381458f8097996ab77780550ba7fb9c240e4799bb781941abb`
