use openidconnect::{
    ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointSet, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest as oidc_reqwest,
};

use openidconnect::EndpointNotSet;

pub type DiscoveredClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Clone)]
pub struct OidcState {
    pub client: DiscoveredClient,
    pub http_client: oidc_reqwest::Client,
}

impl OidcState {
    pub async fn discover(
        issuer_url: &str,
        client_id: &str,
        client_secret: &str,
        redirect_url: &str,
    ) -> Result<Self, String> {
        let issuer = IssuerUrl::new(issuer_url.to_string())
            .map_err(|e| format!("Invalid issuer URL: {e}"))?;

        let http_client = oidc_reqwest::ClientBuilder::new()
            .redirect(oidc_reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

        let provider_metadata =
            CoreProviderMetadata::discover_async(issuer, &http_client)
                .await
                .map_err(|e| format!("OIDC discovery failed: {e}"))?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(client_id.to_string()),
            Some(ClientSecret::new(client_secret.to_string())),
        )
        .set_redirect_uri(
            RedirectUrl::new(redirect_url.to_string())
                .map_err(|e| format!("Invalid redirect URL: {e}"))?,
        );

        Ok(Self { client, http_client })
    }

    pub fn authorization_url(&self) -> (url::Url, CsrfToken, Nonce, PkceCodeVerifier) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf_token, nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        (auth_url, csrf_token, nonce, pkce_verifier)
    }
}
