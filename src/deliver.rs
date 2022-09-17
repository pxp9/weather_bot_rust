use crate::db::Repo;
use crate::open_weather_map::client::WeatherApiClient;
use crate::telegram::client::ApiClient;
use fang::async_trait;
use fang::typetag;
use fang::AsyncQueueable;
use fang::AsyncRunnable;
use fang::DateTime;
use fang::Deserialize;
use fang::FangError;
use fang::Scheduled;
use fang::Serialize;
use fang::Utc;
use typed_builder::TypedBuilder;

pub const SCHEDULED_TASK_TYPE: &str = "scheduled_forecast";

#[derive(Serialize, Deserialize, Debug, TypedBuilder, Eq, PartialEq, Clone)]
#[serde(crate = "fang::serde")]
pub struct ScheduleWeatherTask {
    cron_expression: String,
    username: String,
    chat_id: i64,
    user_id: u64,
    city_id: i32,
}

impl ScheduleWeatherTask {
    fn compute_next_delivery(&self) -> DateTime<Utc> {
        // compute next deliver
        // This unwrap is secure because it depends of a call that i have done.
        // So if here panic! for unwrap may be a bug in the bot.

        Repo::calculate_next_delivery(&self.cron_expression).unwrap()
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for ScheduleWeatherTask {
    async fn run(&self, queueable: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        // Here we should fetch every forecast from forecasts table that
        // created_at <= Utc::now() + Duration::minutes(5)

        let repo = Repo::repo().await?;
        let api = ApiClient::api_client().await;

        let city = repo.search_city_by_id(&self.city_id).await?;

        let next_delivery = self.compute_next_delivery();
        // Insert forecast in forecasts table if not exists or update the forecasts table.

        repo.update_or_insert_forecast(
            &self.chat_id,
            self.user_id,
            &self.city_id,
            self.cron_expression.clone(),
            next_delivery,
        )
        .await?;

        let task = ScheduleWeatherTask::builder()
            .username(self.username.clone())
            .cron_expression(self.cron_expression.clone())
            .chat_id(self.chat_id)
            .user_id(self.user_id)
            .city_id(self.city_id)
            .build();

        queueable.schedule_task(&task).await?;

        let weather_client = WeatherApiClient::weather_client().await;

        let weather_info = weather_client
            .fetch_weekly(city.coord.lat, city.coord.lon)
            .await
            .unwrap();

        let text = format!(
            "Hi {} !, this is your scheduled weather info.\n\n {},{}\nLat {} , Lon {}\n{}",
            self.username, city.name, city.country, city.coord.lat, city.coord.lon, weather_info,
        );

        api.send_message_without_reply(self.chat_id, text)
            .await
            .unwrap();

        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn task_type(&self) -> String {
        SCHEDULED_TASK_TYPE.to_string()
    }

    fn cron(&self) -> Option<Scheduled> {
        Some(Scheduled::ScheduleOnce(self.compute_next_delivery()))
    }
}
