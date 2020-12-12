macro_rules! define_client {
    ($($type: tt),*) => {
        type ChannelHolder = once_cell::sync::Lazy<crate::util::init_once::InitOnce<tonic::transport::Channel>>;
        static CHANNEL: ChannelHolder = once_cell::sync::Lazy::new(|| crate::util::init_once::InitOnce::new());

        pub(crate) async fn init() -> Result<(), Box<dyn std::error::Error>> {
            CHANNEL
            .init(|| async {
                Ok(crate::service::create_channel(DOMAIN).await?)
                as Result<tonic::transport::Channel, tonic::transport::Error>
            })
            .await?;
            Ok(())
        }

        $(
            impl $type<tonic::transport::Channel> {
                async fn get() -> Result<Self, Box<dyn std::error::Error>> {
                    let channel = CHANNEL.get().await.clone();
                    let token = crate::service::get_token(&[SCOPE]).await?;

                    let bearer = format!("Bearer {}", token.as_str());
                    let header_value = tonic::metadata::MetadataValue::from_str(&bearer)?;
                    let client = Self::with_interceptor(channel, move |mut req: tonic::Request<()>| {
                        req.metadata_mut()
                            .insert("authorization", header_value.clone());
                        Ok(req)
                    });
                    Ok(client)
                }
            }
        )*
    };
}
