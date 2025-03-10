use crate::{primitives::Profile, IProfile, IRel, Result, Xplore};
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
impl IRel for Xplore {
    async fn following(
        &self,
        user_id: &str,
        count: i32,
        cursor: Option<String>,
    ) -> Result<(Vec<Profile>, Option<String>)> {
        let response = rel_utils::fetch_profile_following(self, user_id, count, cursor).await?;
        Ok((response.profiles, response.next))
    }

    async fn followers(
        &self,
        user_id: &str,
        count: i32,
        cursor: Option<String>,
    ) -> Result<(Vec<Profile>, Option<String>)> {
        let response = rel_utils::fetch_profile_following(self, user_id, count, cursor).await?;
        Ok((response.profiles, response.next))
    }

    async fn follow(&self, username: &str) -> Result<()> {
        let user_id = self.get_user_id(username).await?;

        let url = "https://api.twitter.com/1.1/friendships/create.json";

        let form = vec![
            ("include_profile_interstitial_type".to_string(), "1".to_string()),
            ("skip_status".to_string(), "true".to_string()),
            ("user_id".to_string(), user_id),
        ];

        let _ = self.inner.rpc.request_form::<Value>(url, username, form).await?;

        Ok(())
    }

    async fn unfollow(&self, username: &str) -> Result<()> {
        let user_id = self.get_user_id(username).await?;

        let url = "https://api.twitter.com/1.1/friendships/destroy.json";

        let form = vec![
            ("include_profile_interstitial_type".to_string(), "1".to_string()),
            ("skip_status".to_string(), "true".to_string()),
            ("user_id".to_string(), user_id),
        ];

        let (_, _) = self.inner.rpc.request_form::<Value>(url, username, form).await?;

        Ok(())
    }
}

mod rel_utils {
    use crate::{
        primitives::{Profile, RelationshipTimeline, TimelineInstruction},
        timeline::{home::get_following_timeline, v1::QueryProfilesResponse},
        Result, Xplore,
    };
    use chrono::{DateTime, Utc};

    pub async fn fetch_profile_following(
        xyz: &Xplore,
        user_id: &str,
        max_profiles: i32,
        cursor: Option<String>,
    ) -> Result<QueryProfilesResponse> {
        let timeline = get_following_timeline(xyz, user_id, max_profiles, cursor).await?;

        Ok(parse_relationship_timeline(&timeline))
    }

    fn parse_relationship_timeline(timeline: &RelationshipTimeline) -> QueryProfilesResponse {
        let mut profiles = Vec::new();
        let mut next_cursor = None;
        let mut previous_cursor = None;

        if let Some(data) = &timeline.data {
            for instruction in &data.user.result.timeline.timeline.instructions {
                match instruction {
                    TimelineInstruction::AddEntries { entries } => {
                        for entry in entries {
                            if let Some(item_content) = &entry.content.item_content {
                                if let Some(user_results) = &item_content.user_results {
                                    if let Some(legacy) = &user_results.result.legacy {
                                        let profile = Profile {
                                            username: legacy.screen_name.clone().unwrap_or_default(),
                                            name: legacy.name.clone().unwrap_or_default(),
                                            id: user_results
                                                .result
                                                .rest_id
                                                .as_ref()
                                                .map(String::from)
                                                .unwrap_or_default(),
                                            description: legacy.description.clone(),
                                            location: legacy.location.clone(),
                                            url: legacy.url.clone(),
                                            protected: legacy.protected.unwrap_or_default(),
                                            verified: legacy.verified.unwrap_or_default(),
                                            followers_count: legacy.followers_count.unwrap_or_default(),
                                            following_count: legacy.friends_count.unwrap_or_default(),
                                            tweets_count: legacy.statuses_count.unwrap_or_default(),
                                            listed_count: legacy.listed_count.unwrap_or_default(),
                                            created_at: legacy
                                                .created_at
                                                .as_ref()
                                                .and_then(|date| {
                                                    DateTime::parse_from_str(date, "%a %b %d %H:%M:%S %z %Y")
                                                        .ok()
                                                        .map(|dt| dt.with_timezone(&Utc))
                                                })
                                                .unwrap_or_default(),
                                            profile_image_url: legacy.profile_image_url_https.clone(),
                                            profile_banner_url: legacy.profile_banner_url.clone(),
                                            pinned_tweet_id: legacy.pinned_tweet_ids_str.clone(),
                                            is_blue_verified: Some(
                                                user_results.result.is_blue_verified.unwrap_or(false),
                                            ),
                                        };

                                        profiles.push(profile);
                                    }
                                }
                            } else if let Some(cursor_content) = &entry.content.cursor {
                                match cursor_content.cursor_type.as_deref() {
                                    Some("Bottom") => next_cursor = Some(cursor_content.value.clone()),
                                    Some("Top") => previous_cursor = Some(cursor_content.value.clone()),
                                    _ => {}
                                }
                            }
                        }
                    }
                    TimelineInstruction::ReplaceEntry { entry } => {
                        if let Some(cursor_content) = &entry.content.cursor {
                            match cursor_content.cursor_type.as_deref() {
                                Some("Bottom") => next_cursor = Some(cursor_content.value.clone()),
                                Some("Top") => previous_cursor = Some(cursor_content.value.clone()),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        QueryProfilesResponse { profiles, next: next_cursor, previous: previous_cursor }
    }
}
