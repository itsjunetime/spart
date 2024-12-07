/*use merde::CowStr;

struct PlayedSong<'file> {
	ts: CowStr<'file>,
	username: CowStr<'file>,
	platform: CowStr<'file>,
	ms_played: u64,
	conn_country: CowStr<'file>,
	ip_addr_decrypted: Option<CowStr<'file>>,
	user_agent_decrypted: Option<CowStr<'file>>,
	master_metadata_track_name: Option<CowStr<'file>>,
	master_metadata_album_artist_name: Option<CowStr<'file>>,
	master_metadata_album_album_name: Option<CowStr<'file>>,
	spotify_track_uri: Option<CowStr<'file>>,
	episode_name: Option<CowStr<'file>>,
	episode_show_name: Option<CowStr<'file>>,
	spotify_episode_uri: Option<CowStr<'file>>,
	reason_start: CowStr<'file>,
	reason_end: Option<CowStr<'file>>,
	shuffle: bool,
	skipped: Option<bool>,
	offline: bool,
	offline_timestamp: u64,
	incognito_mode: bool
}

merde::derive! {
	impl (Deserialize) for struct PlayedSong<'file> {
		ts,
		username,
		platform,
		ms_played,
		conn_country,
		ip_addr_decrypted,
		user_agent_decrypted,
		master_metadata_track_name,
		master_metadata_album_artist_name,
		master_metadata_album_album_name,
		spotify_track_uri,
		episode_name,
		episode_show_name,
		spotify_episode_uri,
		reason_start,
		reason_end,
		shuffle,
		skipped,
		offline,
		offline_timestamp,
		incognito_mode
	}
}*/
