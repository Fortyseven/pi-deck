import './app.scss';
import './styles/transcript.scss';
import { mount } from 'svelte';
import App from './App.svelte';

mount(App, { target: document.getElementById('app') });
