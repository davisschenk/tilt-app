import LegalLayout from "@/components/legal-layout";

// NOTE FOR OPERATORS: starter terms for a hobby service. Not legal advice;
// review and adapt (liability, governing law, acceptable use) before opening
// sign-ups to the public.
export default function Terms() {
  return (
    <LegalLayout title="Terms of use" updated="November 2026">
      <p>
        By creating an account or using TiltBrew you agree to these terms. If you do not agree, please
        do not use the service.
      </p>

      <h2>The service</h2>
      <p>
        TiltBrew is a tool for recording and charting fermentation readings. It is provided as is,
        without warranty of any kind, and may change, be interrupted, or be discontinued. Readings,
        alerts and calculations (including ABV, attenuation and nutrient schedules) are aids, not
        guarantees: sensors drift and alerts can fail to arrive. Do not rely on TiltBrew for anything
        where a missed alert could cause harm, and use your own judgement and equipment for food
        safety.
      </p>

      <h2>Your account and data</h2>
      <p>
        You are responsible for activity under your account and for keeping your API keys secret. You
        keep ownership of the data and photos you add; you give the operator permission to store and
        process them only to run the service for you. You can export or delete your data at any time
        from the Account page.
      </p>

      <h2>Acceptable use</h2>
      <ul>
        <li>Do not attempt to access other people's data or disrupt the service.</li>
        <li>Do not point alert webhooks at systems you do not own or have permission to contact.</li>
        <li>Do not upload content that is unlawful or that you do not have the right to share.</li>
        <li>Do not use automated means to create accounts or overload the service.</li>
      </ul>

      <h2>Suspension and ending</h2>
      <p>
        The operator may suspend or remove accounts that break these terms or put the service at
        risk. You can stop using the service and delete your account at any time.
      </p>

      <h2>Limits of liability</h2>
      <p>
        To the extent the law allows, the operator is not liable for losses arising from use of, or
        inability to use, the service, including lost data or a spoiled batch.
      </p>

      <h2>Changes</h2>
      <p>These terms may be updated; the date at the top shows the latest version.</p>
    </LegalLayout>
  );
}
