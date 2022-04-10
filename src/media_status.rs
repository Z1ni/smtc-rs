use std::time::Duration;

use async_std::channel::Sender;
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionMediaProperties,
        GlobalSystemMediaTransportControlsSessionPlaybackInfo,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus, MediaPropertiesChangedEventArgs,
        PlaybackInfoChangedEventArgs, TimelinePropertiesChangedEventArgs,
    },
};

#[derive(Debug)]
pub enum PlayStatus {
    Unknown,
    Closed,
    Opened,
    Changing,
    Stopped,
    Playing,
    Paused,
}

impl From<PlayStatus> for MediaEvent {
    fn from(status: PlayStatus) -> Self {
        MediaEvent::PlayStatusChanged(status)
    }
}

impl From<GlobalSystemMediaTransportControlsSessionPlaybackStatus> for PlayStatus {
    fn from(status: GlobalSystemMediaTransportControlsSessionPlaybackStatus) -> Self {
        match status {
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => PlayStatus::Closed,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Opened => PlayStatus::Opened,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing => {
                PlayStatus::Changing
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => PlayStatus::Stopped,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => PlayStatus::Playing,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => PlayStatus::Paused,
            _ => PlayStatus::Unknown,
        }
    }
}

impl From<GlobalSystemMediaTransportControlsSessionPlaybackInfo> for PlayStatus {
    fn from(info: GlobalSystemMediaTransportControlsSessionPlaybackInfo) -> Self {
        info.PlaybackStatus().unwrap().into()
    }
}

#[derive(Debug)]
pub struct TrackInfo {
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub track_num: i32,
    pub title: String,
}

impl From<TrackInfo> for MediaEvent {
    fn from(info: TrackInfo) -> Self {
        MediaEvent::InfoChanged(info)
    }
}

impl From<GlobalSystemMediaTransportControlsSessionMediaProperties> for TrackInfo {
    fn from(props: GlobalSystemMediaTransportControlsSessionMediaProperties) -> Self {
        let track_num = props.TrackNumber().unwrap();
        let album = props.AlbumTitle().unwrap().to_string();
        let artist = props.Artist().unwrap().to_string();
        let title = props.Title().unwrap().to_string();
        let album_artist = props.AlbumArtist().unwrap().to_string();

        /* let subtitle = props.Subtitle().unwrap().to_string();
        let genres: Vec<String> = props
            .Genres()
            .unwrap()
            .into_iter()
            .map(|h| h.to_string())
            .collect(); */

        TrackInfo {
            artist,
            album,
            album_artist,
            track_num,
            title,
        }
    }
}

impl TrackInfo {
    pub fn get_current_track() -> Option<TrackInfo> {
        let session_mgr = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .unwrap()
            .get()
            .unwrap();

        match session_mgr.GetCurrentSession() {
            Ok(current_session) => {
                let info: TrackInfo = current_session
                    .TryGetMediaPropertiesAsync()
                    .unwrap()
                    .get()
                    .unwrap()
                    .into();

                Some(info)
            }
            Err(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum MediaEvent {
    PlayStatusChanged(PlayStatus),
    InfoChanged(TrackInfo),
    PositionChanged { current: Duration, length: Duration },
    SourceAppChanged(Option<String>),
}

pub struct WindowsMediaEventListener {
    // The Windows event listeners are dropped when the session manager is dropped, so we keep it alive here
    _session_manager: GlobalSystemMediaTransportControlsSessionManager,
    recv: async_std::channel::Receiver<MediaEvent>,
}

impl WindowsMediaEventListener {
    pub fn new() -> Option<WindowsMediaEventListener> {
        let (media_ev_send, media_ev_recv) = async_std::channel::bounded(20);

        let media_ev_send_clone = media_ev_send.clone();

        let cur_session_changed_ev_handler = TypedEventHandler::new(
            move |session_mgr_opt: &Option<GlobalSystemMediaTransportControlsSessionManager>, _| {
                let current_session_result = session_mgr_opt.as_ref().unwrap().GetCurrentSession();

                if current_session_result.is_err() {
                    // No current session
                    media_ev_send_clone
                        .try_send(MediaEvent::SourceAppChanged(None))
                        .unwrap();
                    return Ok(());
                }

                let current_session = current_session_result.unwrap();

                WindowsMediaEventListener::add_listeners(
                    &current_session,
                    media_ev_send_clone.clone(),
                );

                media_ev_send_clone
                    .try_send(MediaEvent::SourceAppChanged(Some(
                        current_session.SourceAppUserModelId().unwrap().to_string(),
                    )))
                    .unwrap();

                let media_properties = current_session
                    .TryGetMediaPropertiesAsync()
                    .unwrap()
                    .get()
                    .unwrap();

                media_ev_send_clone
                    .try_send(MediaEvent::InfoChanged(media_properties.into()))
                    .unwrap();

                let timeline_props = current_session.GetTimelineProperties().unwrap();
                let current = timeline_props.Position().unwrap().into();
                let length = timeline_props.MaxSeekTime().unwrap().into();

                media_ev_send_clone
                    .try_send(MediaEvent::PositionChanged { current, length })
                    .unwrap();

                Ok(())
            },
        );

        let session_mgr = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .unwrap()
            .get()
            .unwrap();

        session_mgr
            .CurrentSessionChanged(cur_session_changed_ev_handler)
            .unwrap();

        if let Ok(current_session) = session_mgr.GetCurrentSession() {
            // Get the current status
            let status: PlayStatus = current_session.GetPlaybackInfo().unwrap().into();
            let info: TrackInfo = current_session
                .TryGetMediaPropertiesAsync()
                .unwrap()
                .get()
                .unwrap()
                .into();
            let timeline_props = current_session.GetTimelineProperties().unwrap();
            let current = timeline_props.Position().unwrap().into();
            let length = timeline_props.MaxSeekTime().unwrap().into();

            media_ev_send
                .try_send(MediaEvent::SourceAppChanged(Some(
                    current_session.SourceAppUserModelId().unwrap().to_string(),
                )))
                .unwrap();
            media_ev_send.try_send(status.into()).unwrap();
            media_ev_send.try_send(info.into()).unwrap();
            media_ev_send
                .try_send(MediaEvent::PositionChanged { current, length })
                .unwrap();

            WindowsMediaEventListener::add_listeners(&current_session, media_ev_send);
        } else {
            // No current session
        }

        Some(WindowsMediaEventListener {
            _session_manager: session_mgr,
            recv: media_ev_recv,
        })
    }

    pub fn get_media_events(&self) -> async_std::channel::Receiver<MediaEvent> {
        self.recv.clone()
    }

    fn add_listeners(
        session: &GlobalSystemMediaTransportControlsSession,
        send: Sender<MediaEvent>,
    ) {
        session
            .MediaPropertiesChanged(
                WindowsMediaEventListener::create_media_prop_changed_ev_handler(send.clone()),
            )
            .unwrap();

        session
            .TimelinePropertiesChanged(
                WindowsMediaEventListener::create_timeline_prop_changed_ev_handler(send.clone()),
            )
            .unwrap();

        session
            .PlaybackInfoChanged(
                WindowsMediaEventListener::create_playback_info_changed_ev_handler(send.clone()),
            )
            .unwrap();
    }

    fn to_duration(ts: windows::Foundation::TimeSpan) -> std::time::Duration {
        // TimeSpan Duration is "The time interval, in 100-nanosecond units." as per the WinAPI docs
        let duration_ns = ts.Duration as u64 * 100;
        //ts.Duration * 100 / 1_000_000_000
        Duration::from_nanos(duration_ns)
    }

    fn create_media_prop_changed_ev_handler(
        ev_sender: async_std::channel::Sender<MediaEvent>,
    ) -> TypedEventHandler<GlobalSystemMediaTransportControlsSession, MediaPropertiesChangedEventArgs>
    {
        TypedEventHandler::new(
            move |session_opt: &Option<GlobalSystemMediaTransportControlsSession>, _| {
                let session = session_opt.as_ref().unwrap();

                let media_properties = session.TryGetMediaPropertiesAsync().unwrap().get().unwrap();

                ev_sender
                    .try_send(MediaEvent::InfoChanged(media_properties.into()))
                    .unwrap();

                Ok(())
            },
        )
    }

    fn create_timeline_prop_changed_ev_handler(
        ev_sender: async_std::channel::Sender<MediaEvent>,
    ) -> TypedEventHandler<
        GlobalSystemMediaTransportControlsSession,
        TimelinePropertiesChangedEventArgs,
    > {
        TypedEventHandler::new(
            move |session_opt: &Option<GlobalSystemMediaTransportControlsSession>, _| {
                let session = session_opt.as_ref().unwrap();

                let timeline_properties = session.GetTimelineProperties().unwrap();

                let position =
                    WindowsMediaEventListener::to_duration(timeline_properties.Position().unwrap());
                //let min_seek_time = to_duration(timeline_properties.MinSeekTime().unwrap());
                let max_seek_time = WindowsMediaEventListener::to_duration(
                    timeline_properties.MaxSeekTime().unwrap(),
                );

                ev_sender
                    .try_send(MediaEvent::PositionChanged {
                        current: position,
                        length: max_seek_time,
                    })
                    .unwrap();

                Ok(())
            },
        )
    }

    fn create_playback_info_changed_ev_handler(
        ev_sender: async_std::channel::Sender<MediaEvent>,
    ) -> TypedEventHandler<GlobalSystemMediaTransportControlsSession, PlaybackInfoChangedEventArgs>
    {
        TypedEventHandler::new(
            move |session_opt: &Option<GlobalSystemMediaTransportControlsSession>, _| {
                let session = session_opt.as_ref().unwrap();

                let playback_info = session.GetPlaybackInfo().unwrap();

                let status = match playback_info.PlaybackStatus().unwrap() {
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => {
                        PlayStatus::Closed
                    }
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Opened => {
                        PlayStatus::Opened
                    }
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing => {
                        PlayStatus::Changing
                    }
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => {
                        PlayStatus::Stopped
                    }
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => {
                        PlayStatus::Playing
                    }
                    GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => {
                        PlayStatus::Paused
                    }
                    _ => PlayStatus::Unknown,
                };

                ev_sender
                    .try_send(MediaEvent::PlayStatusChanged(status))
                    .unwrap();

                Ok(())
            },
        )
    }
}
