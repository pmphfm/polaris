# Playlists

## Load

Loading a playlist may lead to soft errors.

The version of `Polaris` continues to load a playlist even when songs in the
playlist have missing audio files. For those missing songs, playlist returned
will contain

- the original missing path with "-not-found.mp3" appended
- the artist set to "error artist"
- album set to "error album".

When such a stale entry is played, the client might skip or beep. This will help
the users know that they have a stale entry in the playlist.  Seeing a stale
entry in the playlist, user can take a corrective action, two of which are

- restore the original song at the path.
- remove the song from the playlist and save the playlist.
- remove stale entry and add corrected/updated song path.

### Rationale

Changing the on-disk path of songs may lead to entries in user created playlists
becoming stale. Playlists become extremely difficult to manage especially
when a **large** and **actively maintained**  songs collection is reorganized.

While automatically fixing/updating stale entries is a good feature to have,
reporting stale entries will be helpful for the users/admin to fix stale entries
manually.

In the existing `Polaris` clients, there is no good infrastructure to report
stale entries to users or to an admin. So we revert to reporting issues when a
playlist is loaded/played. Another option is to not load the playlist at all.
Instead log the stale entries. This is not great since

- One stale entry makes the entire playlist useless
- Only admins have "access" to logs and leaves
  - Users cannot take corrective actions
  - Makes being an admin a lot more harder.

## Export

Polaris supports exporting playlists in [m3u](https://en.wikipedia.org/wiki/M3U)
format.

When exporting a playlist, polaris adds custom M3U headers called
`EXT-X-POLARIS`. One directive under such header is `COMMON_PATH`, which when
set, contains a common prefix string that is found among *all* the songs in the
playlist. This string can help users place the exported m3u file in their
filesystem.

For example, placing the following m3u playlist in the directory that contains
two files (`Khemmis/Hunted/01 - Above The Water.mp3` and
`Tobokegao/Picnic (Remixes)/01 - ピクニック (Picnic) (Remix).mp3`) will make m3u
work without any errors when played through external players.

```m3u
#EXTM3U
#EXT-X-POLARIS: COMMON_PATH=/mnt/test-data/small-collection/
Khemmis/Hunted/01 - Above The Water.mp3
Tobokegao/Picnic (Remixes)/01 - ピクニック (Picnic) (Remix).mps
```

From polaris point of view, those two files are found in
`/mnt/test-data/small-collection/` as indicated by `EXT-X-POLARIS: COMMON_PATH`.

Note:

1. Any line starting with a `#` in a `.m3u` file can be ignored.
2. Polaris may add additional extended m3u directives in future in
exported files.
