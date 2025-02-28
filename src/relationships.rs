use crate::error::Result;
use crate::primitives::Profile;
use crate::primitives::RelationshipTimeline;
use crate::primitives::TimelineInstruction;
use crate::timeline::v1::QueryProfilesResponse;
use crate::IXYZProfile;
use crate::XYZ;
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde_json::{json, Value};

impl XYZ {
    pub async fn get_following(
        &self,
        user_id: &str,
        count: i32,
        cursor: Option<String>,
    ) -> Result<(Vec<Profile>, Option<String>)> {
        let response = self.fetch_profile_following(user_id, count, cursor).await?;
        Ok((response.profiles, response.next))
    }
    pub async fn get_followers(
        &self,
        user_id: &str,
        count: i32,
        cursor: Option<String>,
    ) -> Result<(Vec<Profile>, Option<String>)> {
        let response = self.fetch_profile_following(user_id, count, cursor).await?;
        Ok((response.profiles, response.next))
    }

    pub async fn fetch_profile_following(
        &self,
        user_id: &str,
        max_profiles: i32,
        cursor: Option<String>,
    ) -> Result<QueryProfilesResponse> {
        let timeline = self.get_following_timeline(user_id, max_profiles, cursor).await?;

        Ok(Self::parse_relationship_timeline(&timeline))
    }

    async fn get_following_timeline(
        &self,
        user_id: &str,
        max_items: i32,
        cursor: Option<String>,
    ) -> Result<RelationshipTimeline> {
        let count = if max_items > 50 { 50 } else { max_items };

        let mut variables = json!({
            "userId": user_id,
            "count": count,
            "includePromotedContent": false,
        });

        if let Some(cursor_val) = cursor {
            if !cursor_val.is_empty() {
                variables["cursor"] = json!(cursor_val);
            }
        }

        let features = json!({
            "responsive_web_twitter_article_tweet_consumption_enabled": false,
            "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled": true,
            "longform_notetweets_inline_media_enabled": true,
            "responsive_web_media_download_video_enabled": false,
        });

        let url = format!(
            "https://twitter.com/i/api/graphql/iSicc7LrzWGBgDPL0tM_TQ/Following?variables={}&features={}",
            urlencoding::encode(&variables.to_string()),
            urlencoding::encode(&features.to_string())
        );

        let (data, _) = self.inner.rpc.send_request::<RelationshipTimeline>(&url, Method::GET, None).await?;

        Ok(data)
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

    pub async fn follow_user(&self, username: &str) -> Result<()> {
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

    pub async fn unfollow_user(&self, username: &str) -> Result<()> {
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
