import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

import { PROJECT_URL } from "@/lib/app-info";


function AboutSection() {
  const [version, setVersion] = useState("…");

  useEffect(() => {
    let active = true;
    getVersion()
      .then((value) => {
        if (active) setVersion(value);
      })
      .catch(() => {
        if (active) setVersion("未知");
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="settings-about">
      <div className="settings-about__row">
        <span className="settings-about__label">应用</span>
        <span className="settings-about__value">StepDoc</span>
      </div>
      <div className="settings-about__row">
        <span className="settings-about__label">版本</span>
        <span className="settings-about__value">{version}</span>
      </div>
      <div className="settings-about__row">
        <span className="settings-about__label">项目地址</span>
        {PROJECT_URL ? (
          <button
            type="button"
            className="settings-about__link"
            onClick={() => void openUrl(PROJECT_URL).catch((error) => alert(String(error)))}
          >
            {PROJECT_URL}
          </button>
        ) : (
          <span className="settings-about__empty">暂未设置</span>
        )}
      </div>
    </div>
  );
}

export { AboutSection };
