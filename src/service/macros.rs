macro_rules! define_client {
    ($($type: tt),*) => {
        struct ChannelCreator {}
        #[async_trait::async_trait]
        impl crate::util::init_once::AsyncCreator<tonic::transport::Channel> for ChannelCreator {
            async fn create(&self) -> Result<tonic::transport::Channel, Box<dyn std::error::Error>> {
                Ok(crate::service::create_channel(DOMAIN).await?)
            }
        }

        type ChannelHolder = once_cell::sync::Lazy<
            crate::util::init_once::InitOnce<tonic::transport::Channel, ChannelCreator>,
        >;
        static CHANNEL: ChannelHolder =
            once_cell::sync::Lazy::new(|| crate::util::init_once::InitOnce::new(ChannelCreator {}));

        $(
            impl $type<tonic::transport::Channel> {
                async fn get() -> Result<Self, Box<dyn std::error::Error>> {
                    let channel = CHANNEL.get().await?.clone();
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
