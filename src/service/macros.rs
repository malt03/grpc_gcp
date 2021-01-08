macro_rules! define_client {
    ($($type: tt),*) => {
        struct ChannelInitializer {}
        #[async_trait::async_trait]
        impl crate::util::init_once::AsyncInitializer for ChannelInitializer {
            type T = tonic::transport::Channel;
            type Error = tonic::transport::Error;
            async fn create(&self) -> Result<tonic::transport::Channel, tonic::transport::Error> {
                Ok(crate::service::create_channel(DOMAIN).await?)
            }
        }

        type ChannelHolder =
            once_cell::sync::Lazy<crate::util::init_once::AsyncInitOnce<ChannelInitializer>>;
        static CHANNEL: ChannelHolder = once_cell::sync::Lazy::new(|| {
            crate::util::init_once::AsyncInitOnce::new(ChannelInitializer {})
        });

        $(
            impl $type<tonic::transport::Channel> {
                pub(crate) async fn get() -> Result<Self, Error> {
                    let channel = CHANNEL.get().await?.clone();
                    let token = crate::service::get_token(&[SCOPE]).await?;

                    let bearer = format!("Bearer {}", token.as_str());
                    let header_value = tonic::metadata::MetadataValue::from_str(&bearer).unwrap();
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

macro_rules! common_panic {
    () => {
        panic!("An unexpected error has occured! Please report to issue. https://github.com/malt03/grpc_gcp/issues")
    }
}
