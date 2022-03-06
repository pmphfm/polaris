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

## Rationale

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
