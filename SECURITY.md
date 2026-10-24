# Security

NPZSee opens potentially untrusted NPY/NPZ files read only. Object arrays are not
unpickled or executed. Header sizes, shapes, byte counts and ZIP CRCs are checked.
Decoding can require temporary disk space proportional to a selected array.

Report suspected vulnerabilities through GitHub's private vulnerability reporting:
https://github.com/terenceyjh/npzsee/security/advisories/new

Do not attach private recordings or credentials. A small synthetic reproducer,
version, operating system and the exact error are usually sufficient. For ordinary
bugs use https://github.com/terenceyjh/npzsee/issues.
