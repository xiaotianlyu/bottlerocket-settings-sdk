# NTP settings

Existing URL-string lists continue to use `pool` with shared `settings.ntp.options`:

```toml
[settings.ntp]
time-servers = ["169.254.169.123", "2.amazon.pool.ntp.org"]
options = ["iburst"]
```

Per-source configuration uses a list of objects. Customers can opt in with TOML
arrays of tables:

```toml
[[settings.ntp.time-servers]]
address = "169.254.169.123"
directive = "server"
options = ["prefer", "iburst", "minpoll 4", "maxpoll 4"]

[[settings.ntp.time-servers]]
address = "time.aws.com"
directive = "pool"
options = ["iburst"]
```

Each object requires `address`. An omitted `directive` renders as `server`;
omitted `options` renders no options. Top-level shared options apply to URL
strings only. Mixed string/object lists are rejected.

Both list forms are stored at the same single datastore key. Setting the list
to `[]` explicitly configures no sources. Updating an individual source requires
submitting the complete list.

The earlier named-map form remains a compatibility input for complete entries.
Serialization orders entries by name and writes an object list, dropping the
names. Readback is a list. A named-map write replaces the entire list; it does
not merge individual named entries. Partial named entries without an address
are rejected. Datastores containing experimental named child keys require a
separate transition and are outside the legacy-list upgrade migration.

The default configuration remains the legacy URL list for fresh and upgraded
nodes. Existing stored lists are preserved on upgrade.

Rollback to the legacy model preserves addresses and complete options shared by
all sources. The old template renders those addresses as `pool`, and per-source
directives and unique options are lost. Empty common options become an explicit
`settings.ntp.options = []`. Unknown or conflicting option syntax has an empty
shared projection rather than producing orphaned or incorrectly paired arguments.

Optional `settings.ntp.logging` lists chrony log categories. Empty or omitted
logging produces no `log` directive; rollback removes the setting.
