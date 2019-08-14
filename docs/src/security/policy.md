# Policy

Safety is one of the core principles of what we do, and to that end, we would like to ensure that async-std has a secure implementation. Thank you for taking the time to responsibly disclose any issues you find.

All security bugs in async-std distribution should be reported by email to security@ferrous-systems.com. This list is delivered to a small security team. Your email will be acknowledged within 24 hours, and you’ll receive a more detailed response to your email within 48 hours indicating the next steps in handling your report. If you would like, you can encrypt your report using our public key. This key is also On MIT’s keyserver and reproduced below.

Be sure to use a descriptive subject line to avoid having your report be missed. After the initial reply to your report, the security team will endeavor to keep you informed of the progress being made towards a fix and full announcement. As recommended by [RFPolicy][rf-policy], these updates will be sent at least every five days. In reality, this is more likely to be every 24-48 hours.

If you have not received a reply to your email within 48 hours, or have not heard from the security team for the past five days, there are a few steps you can take (in order):

* Contact the current security coordinator TODO directly.
* Contact the back-up contact TODO directly.
* Post on our Community forums

Please note that the discussion forums are public areas. When escalating in these venues, please do not discuss your issue. Simply say that you’re trying to get a hold of someone from the security team.

[rf-policy]: https://en.wikipedia.org/wiki/RFPolicy

## Disclosure policy

The async-std project has a 5 step disclosure process.

* The security report is received and is assigned a primary handler. This person will coordinate the fix and release process.
* The problem is confirmed and a list of all affected versions is determined.
* Code is audited to find any potential similar problems.
* Fixes are prepared for all releases which are still under maintenance. These fixes are not committed to the public repository but rather held locally pending the announcement.
* On the embargo date, the changes are pushed to the public repository and new builds are deployed to crates.io. Within 6 hours, a copy of the advisory will be published on the the async.rs blog.

This process can take some time, especially when coordination is required with maintainers of other projects. Every effort will be made to handle the bug in as timely a manner as possible, however it’s important that we follow the release process above to ensure that the disclosure is handled in a consistent manner.

## Credits

This policy is adapted from the [Rust project](https://www.rust-lang.org/policies/security) security policy.

## PGP Key

TODO