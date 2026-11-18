import LegalLayout from "@/components/legal-layout";

// NOTE FOR OPERATORS: this text describes what the TiltBrew software itself
// stores and does. It is not legal advice. Review it against your own hosting,
// logging, analytics and jurisdiction, and edit it before opening sign-ups.
export default function Privacy() {
  return (
    <LegalLayout title="Privacy" updated="November 2026">
      <p>
        TiltBrew records fermentation data from your hydrometer. This page says what is stored, who
        can see it, and how to get it back or remove it.
      </p>

      <h2>What we store</h2>
      <ul>
        <li>
          <strong>Your account:</strong> the identifier, email address and display name provided by
          the sign-in provider when you log in.
        </li>
        <li>
          <strong>Your brewing data:</strong> hydrometers, brews and their details, gravity and
          temperature readings, brew log entries, and any photos you upload.
        </li>
        <li>
          <strong>Alert settings:</strong> the webhook addresses and optional authorization header you
          configure, and your alert rules.
        </li>
        <li>
          <strong>API keys:</strong> only a one-way hash of each key and its first characters, never
          the key itself.
        </li>
        <li>
          <strong>Sessions:</strong> a signed, HTTP-only cookie that keeps you logged in, and a
          matching session record on the server. This is the only cookie the application sets.
        </li>
      </ul>

      <h2>Who can see it</h2>
      <p>
        Your brewing data is private to your account: other users cannot read, change or delete it.
        The operator of this instance has technical access to the database. When one of your alert
        rules fires, the alert details (brew name, reading and rule) are sent to the webhook address
        you configured, which is a third-party service you chose.
      </p>

      <h2>Your choices</h2>
      <ul>
        <li>
          <strong>Export:</strong> download everything in your account as a JSON file from the{" "}
          <em>Account</em> page. Secrets (API key hashes, webhook authorization headers) are left out.
        </li>
        <li>
          <strong>Delete:</strong> permanently delete your account and all of its data, including
          uploaded photos, from the same page. This cannot be undone.
        </li>
        <li>
          <strong>Photos and entries:</strong> delete individual brews, log entries or photos at any
          time.
        </li>
      </ul>

      <h2>Retention</h2>
      <p>
        Data is kept until you delete it or delete your account. Deleted data is removed from the
        live database immediately; copies in operator backups may persist until those backups expire.
      </p>

      <h2>Server logs</h2>
      <p>
        Like most web services, the server keeps operational logs of requests (for example paths and
        status codes) to run and secure the service.
      </p>
    </LegalLayout>
  );
}
